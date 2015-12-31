#include "table.h"
#include<iostream>
#define DEBUG 1
int main(int argc, char* argv[]){
    if(argc != 2){
        cout << "Usage: \"" << "WikiLinkr.exe" << " path_to_parsed_file.txt\"" << endl;
        cout <<"(You must first run \"python parsr8.py path_to_wikipedia_dump.xml path_to_parsed_file.txt\")" << endl;
        exit(1);
    }
    //Table t((char*)"../misc_data/simplewiki-20151020_out2");
    Table t(argv[1]);
    string s = "test";
    list<unsigned int> l;
    Entry e(s, l);
    Entry f(s);
    cout << t.resolve_collisions(s) << endl;
    cout << t.resolve_collisions(s) << endl;


    return 0;
}