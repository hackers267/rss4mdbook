use crate::inv::util;

pub fn set(book: String, path: String) {
    if book == "book" {
        println!("upd..env => {}={}", util::ENV_BOOK, &path);
        util::upd_denv(util::ENV_BOOK, &path);
    } else {
        println!(
            r#" ALERT! cfg command only support 1 option :
$ rss4mdbook cfg book path/2/u/mdbook/book,toml

means:
    ~> point the book.toml of your local mdBook site
"#
        );
    }
}
