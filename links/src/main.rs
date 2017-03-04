//#![allow(dead_code)]
//#![feature(plugin, custom_derive, custom_attribute)]

// NOTE: when scaling, remember to change bool link_db/parse/regexes.rs/IS_SIMPLE

// LOGGING
#[macro_use] extern crate slog;
extern crate slog_term;
// SERIALIZING
#[macro_use] extern crate serde_derive;
extern crate serde_json;
extern crate csv;
// MISC
#[macro_use] extern crate clap;
extern crate fnv;
extern crate chrono;
// DATABASE
#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_codegen;
extern crate dotenv;

// COMPONENTS
pub mod link_state;
pub mod cache;

// use std:: 

use clap::Arg;
fn argv<'a>() -> clap::ArgMatches<'a> {
    clap::App::new(crate_name!()).about(crate_description!())
        .author(crate_authors!("\n")).version(crate_version!())
        .arg(Arg::with_name("ranks").short("r").long("ranks_dump").takes_value(true)
             .help("Supply location of rank data in csv form"))
        .arg(Arg::with_name("manifest").short("json").takes_value(true)
             .help("Supply dump of parsed link data manifest in json form"))
        .arg(Arg::with_name("page.sql").short("pages").takes_value(true)
             .conflicts_with("manifest").requires("redirect.sql").requires("pagelinks.sql"))
        .arg(Arg::with_name("redirect.sql").short("dirs").takes_value(true)
             .conflicts_with("manifest").requires("page.sql").requires("pagelinks.sql"))
        .arg(Arg::with_name("pagelinks.sql").short("links").takes_value(true)
             .conflicts_with("manifest").requires("page.sql").requires("redirect.sql"))
        .group(clap::ArgGroup::with_name("sources").required(true)
               .args(&["sql", "manifest", "page.sql", "redirects.sql", "pagelink.sql"]))
        .get_matches()
}

fn main() {
    use link_state::{LinkState, HashLinks};
    //let pages_db = PathBuf::from("/home/owen/wikidata/simplewiki-20170201-page.sql");
    //let redir_db = PathBuf::from("/home/owen/wikidata/simplewiki-20170201-redirect.sql");
    //let links_db = PathBuf::from("/home/owen/wikidata/simplewiki-20170201-pagelinks.sql");
    //let output = PathBuf::from("/home/owen/wikidata/dumps/simple_20170201_dump2");
    //let rank_file = Path::new("/home/owen/wikidata/dumps/simple_20170201_ranks1");
    //let p_rank_file = Path::new("/home/owen/wikidata/dumps/simple_20170201_pretty_ranks1");
    //let dump = PathBuf::from("/home/owen/wikidata/dumps/simple_20170201_dump1");

    //let ld = LinkState::<LinkData>::from_file(dump, new_logger()).unwrap();
    //let rd = LinkState::<RankData>::from_ranks(ld, rank_file);
    //rd.pretty_ranks(p_rank_file).unwrap();
    
    // ====

    let hl = LinkState::<HashLinks>::from_args(argv());
    println!("Size: {}", hl.size());

    //println!("{:?}", hl.bfs(232327,460509));
    hl.print_bfs(232327,460509);

    //thread::sleep(time::Duration::from_secs(30));
}

//fn main() {}
