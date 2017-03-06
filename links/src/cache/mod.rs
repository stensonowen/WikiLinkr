
use {r2d2, diesel};
use dotenv::dotenv;
use diesel::prelude::*;
use diesel::pg::PgConnection;
use r2d2_diesel::ConnectionManager;
use chrono::offset::utc::UTC;
use rocket::http::Status;
use rocket::request::{self, FromRequest};
use rocket::{Request, State, Outcome};
use fnv::FnvHashMap;

use cache::models::*;
use super::link_state::hash_links::Path;
use super::link_state::Entry;
use super::web::CacheSort;

use std::ops::Deref;
use std::env;

pub mod schema;
pub mod models;

const PREVIEW_SIZE: u32 = 16;


// NOTE: most of the db state stuff stolen from Rocket 'todo' example

pub fn establish_connection() -> PgConnection {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("set DATABASE_URL");
    PgConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url))
}

pub type Pool = r2d2::Pool<ConnectionManager<PgConnection>>;

pub fn init_pool() -> Pool {
    dotenv().ok();
    let db_url = env::var("DATABASE_URL").expect("missing DATABASE_URL");
    let config = r2d2::Config::default();
    let manager = ConnectionManager::<PgConnection>::new(db_url);
    r2d2::Pool::new(config, manager).expect("db pool")
}

pub struct Conn(r2d2::PooledConnection<ConnectionManager<PgConnection>>);

impl Deref for Conn {
    type Target = PgConnection;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for Conn {
    type Error = ();
    fn from_request(request: &'a Request<'r>) -> request::Outcome<Conn, ()> {
        let pool = match <State<Pool> as FromRequest>::from_request(request) {
            Outcome::Success(pool) => pool,
            Outcome::Failure(e) => return Outcome::Failure(e),
            Outcome::Forward(_) => return Outcome::Forward(())
        };
        match pool.get() {
            Ok(conn) => Outcome::Success(Conn(conn)),
            Err(_) => Outcome::Failure((Status::ServiceUnavailable, ()))
        }
    }
}


pub fn get_cache<'a>(conn: &PgConnection, links: &'a FnvHashMap<u32,Entry>,
                     sort: &CacheSort, num: u32) 
        -> Option<Vec<(&'a str, i8, &'a str)>> 
{
    use self::schema::paths::dsl::paths;
    use self::schema::paths::dsl as path_row;
    use super::web::CacheSort::*;
    let n = num as i64;
    let rows_res: Result<Vec<DbPath>,diesel::result::Error> = match sort {
        &Recent => paths.order(path_row::timestamp.desc()).limit(n).load(conn),
        &Popular => paths.order(path_row::count.desc()).limit(n).load(conn),
        &Length => paths.order(path_row::result.desc()).limit(n).load(conn),
        /*&Random => {
            extern crate rand;
            let r = rand::random::<u32>() % 53_000_000;
            paths.filter(
            unimplemented!()
        },
        */
    };
    let rows = match rows_res {
        Ok(r) => r,
        Err(_) => return None,
    };
    let mut cache = Vec::<(&str, i8, &str)>::with_capacity(num as usize);
    for db_path in rows.into_iter() {
        let src = db_path.src as u32;
        let dst = db_path.dst as u32;
        let res = db_path.result;
        if let (Some(s), Some(d)) = (links.get(&src), links.get(&dst)) {
            if res > 0 {
                cache.push((&s.title, res as i8 - 1, &d.title));
            }
        }
    }
    Some(cache)
}


//  ---------GETTERS*---------


pub fn lookup_path(conn: &PgConnection, src: u32, dst: u32) -> Option<DbPath> {
    // NOTE: MODIFIES INTERNAL STATE: update count and timestamp
    // NOTE: failure is recoverable
    use self::schema::paths::dsl::paths;
    use self::schema::paths::dsl as path_row;
    let target = paths.find((src as i32, dst as i32));
    diesel::update(target)
        .set(path_row::timestamp.eq(UTC::now()))
        .get_result::<DbPath>(conn) 
        .ok()
        .and_then(|path| diesel::update(target)
                  .set(path_row::count.eq(path.count+1))
                  .get_result(conn)
                  .ok())
}

use super::web;
//fn lookup_addr(conn: &PgConnection, query: &str) -> Result<u32,Vec<String>> {
pub fn lookup_addr<'a>(conn: &PgConnection, query: &'a str) -> web::Node<'a> {
    // return address corresponding to 
    // uhh, will diesel stop this from being a potential sql injection?
    // NOTE: failure is irrecoverable (panic!s)
    use self::schema::titles::dsl::titles;
    use self::schema::titles::dsl as title_row;
    if query.is_empty() {
        web::Node::Unused
    }
    else if let Ok(DbAddr { page_id: id, .. }) = titles.find(query).first(conn) {
        // first try the exact query
        web::Node::Found(id as u32, query)
        //Ok(id as u32)
    } else {
        // would it be super expensive to order by pagerank?
        let fuzzy_query = format!("%{}%", query);
        let guesses = titles.filter(title_row::title.like(fuzzy_query))
            .limit(10).load::<DbAddr>(conn).unwrap();
        //Err(guesses.into_iter().map(|i| i.title).collect())
        if guesses.is_empty() {
            web::Node::Unknown(query)
        } else {
            let sugg: Vec<_> = guesses.into_iter().map(|i| i.title).collect();
            //web::Node::Sugg(guesses.into_iter().map(|i| &i.title).collect())
            //web::Node::Unknown(query)
            web::Node::Sugg(sugg)
        }
    }
}


//  ----------SETTERS---------


pub fn insert_path(conn: &PgConnection, p: Path) -> Option<DbPath> {
    // NOTE: failure is recoverable
    use self::schema::paths;
    let new_path: DbPath = p.into();
    diesel::insert(&new_path).into(paths::table).get_result(conn).ok()
}

fn insert_title(conn: &PgConnection, t: String, n: u32) -> DbAddr {
    // NOTE: won't be run in production, fine to panic!
    // TODO: probably just insert a bunch at a time
    //  the allocation is probably worth it
    use self::schema::titles;
    //let new_addr: DbAddr = (t,n).into();
    let new_addr = DbAddr::blank(t, n);
    diesel::insert(&new_addr).into(titles::table).get_result(conn).unwrap()
}


//  --------TITLES-MGMT--------


//pub fn purge_cac
pub fn populate_addrs(conn: &PgConnection, 
                  links: &FnvHashMap<u32,Entry>,
                  ranks: &FnvHashMap<u32,f64>)
    -> Result<(),diesel::result::Error>
{
    use self::schema::titles;
    use std::cmp::min;
    //could use new struct w/ a &str rather than String
    //fine that this involves a lot of copying
    //just a preproc step; it doesn't have to be performant
    let rows: Vec<_> = links.iter().map(|(&i,e)| DbAddr {
        title: e.title.clone(),
        page_id: i as i32,
        //pagerank: None,
        pagerank: ranks.get(&i).map(|f| *f),
    }).collect();
    // can't insert more than <65k at a time
    let mut current = 0;
    let incr = 10_000;
    while current < rows.len() {
        let upper = min(current+incr, rows.len());
        diesel::insert(&rows[current..upper])
            .into(titles::table)
            .execute(conn)?;
        current += incr;
    }
    Ok(())
    //diesel::insert(&rows[0..20]).into(titles::table).execute(conn)
}


//  ---------LINKS-MGMT--------


fn purge_cache(conn: &PgConnection) -> Result<usize, diesel::result::Error> {
    use self::schema::paths::dsl::paths;
    use self::schema::paths::dsl as path_row;
    println!("WARNING: purging cache");
    //better way to select all?
    diesel::delete(paths.filter(path_row::src.gt(-1))).execute(conn)
}
