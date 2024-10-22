#![allow(unused)]
use std::fs;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use chrono::prelude::*;
use chrono::DateTime;
use rss::Channel;
use rss::Item;
use toml::Value;
use walkdir::{DirEntry as WalkDirEntry, WalkDir};

use crate::inv::util;

/* CLI for gen. RSS from mdBook
    - walk the src path
    - check all .md file's update date
    - order pick lated 5
    - export as rss.xml -> u want path
*/
pub fn exp(book: String) {
    let pkg_name = option_env!("CARGO_PKG_NAME").unwrap_or("DAMA's Crate");
    let pkg_version = option_env!("CARGO_PKG_VERSION").unwrap_or("0.1.42");
    println!(
        "digging and generating by\n\t~> {} v{} <~",
        pkg_name, pkg_version
    );
    println!("let's make RSS now...");
    match read_file(&book) {
        Ok(contents) => {
            let toml_value = contents.parse::<Value>().unwrap();
            let src = toml_value["book"]["src"].as_str().unwrap_or("src");
            let build_dir = toml_value["build"]["build-dir"].as_str().unwrap_or("book");
            match toml_value
                .get("rss4mdbook")
                .and_then(|v| v.get("url-base").and_then(Value::as_str))
            {
                Some(rss_url_base) => {
                    // url-base 存在，并且是字符串类型
                    println!("Found url-base: {}", rss_url_base);

                    let rss_title = toml_value
                        .get("rss4mdbook")
                        .and_then(|v| v.get("rss_title"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("RSS TITLE not define in book.toml");
                    let rss_desc = toml_value
                        .get("rss4mdbook")
                        .and_then(|v| v.get("rss_desc"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("RSS TITLE not define in book.toml");
                    let book_path = Path::new(&book);
                    let src_path = book_path
                        .parent()
                        .map(|path| path.join(src))
                        .and_then(|path| path.to_str().map(|v| v.to_string()));
                    let export_path = book_path
                        .parent()
                        .map(|path| path.join(build_dir))
                        .and_then(|path| path.to_str().map(|v| v.to_string()));
                    let export_rss_path = book_path
                        .parent()
                        .map(|path| path.join("book/RSS.xml"))
                        .and_then(|path| path.to_str().map(|v| v.to_string()));
                    if let (Some(src2md), Some(expath), Some(exprss)) =
                        (src_path, export_path, export_rss_path)
                    {
                        let latest5files = scan_dir(src2md.clone(), 4);
                        println!("will export these article into RSS.xml");
                        match rss4top5md(
                            (&rss_url_base).to_string(),
                            exprss.clone(),
                            src2md.clone(),
                            (&rss_title).to_string(),
                            (&rss_desc).to_string(),
                            latest5files,
                        ) {
                            Ok(_) => println!("\n Export => {}\n\n", exprss.clone()),
                            Err(e) => println!("Error: {}", e),
                        }
                    }
                }
                None => {
                    // url-base 不存在或不是字符串类型
                    println!(
                        r#"Warning: 
[rss4mdbook] not config in mdBook's book.toml, please append such as:

    [rss4mdbook]
    url-base = "https://rs.101.so" # u site's root URL
    "#
                    );
                    std::process::exit(1);
                }
            }
        }
        Err(e) => println!("Error: {}", e),
    }
}

fn scan_dir(src2md: String, topn: usize) -> Vec<String> {
    let walker = WalkDir::new(src2md).into_iter();
    let mut file_modified_times = walker
        .filter_map(Result::ok)
        .filter(|e| !is_hidden(e))
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "md"))
        .filter_map(|e| {
            fs::metadata(e.path())
                .ok()
                .and_then(|m| m.modified().ok().map(|t| (e.path().to_owned(), t)))
        })
        .collect::<Vec<_>>();

    // 排序
    file_modified_times.sort_by_key(|(_, time)| *time);

    // 获取最新的5个文件，过滤掉包含 SUMMARY.md 的路径
    let newest_files: Vec<String> = file_modified_times
        .iter()
        .rev()
        .filter(|(path, _)| !path.to_string_lossy().contains("SUMMARY.md"))
        .take(topn) //.take(5)
        .map(|(path, _)| path.to_string_lossy().to_string())
        .collect();
    newest_files
}

fn rss4top5md(
    uri: String,
    rssfile: String,
    src2md: String,
    rss_title: String,
    rss_desc: String,
    latest5files: Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    // 创建一个 RSS channel
    let mut channel = Channel {
        link: uri.clone(),
        title: rss_title,
        description: rss_desc,
        generator: Some("my_rss_generator".to_owned()),
        ..Default::default()
    };

    // 为每个文件创建 RSS item
    for file in latest5files {
        let _p4src = site_uri(file.clone(), &src2md);
        let _uri4md = &_p4src[.._p4src.len() - 3];
        let metadata = fs::metadata(&file)?;
        let date = DateTime::<Local>::from(metadata.modified()?).to_rfc2822();
        let content = fs::read_to_string(&file)?;
        let file_path = PathBuf::from(&file);
        let file_name = file_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let item = Item {
            title: Some(file_name),
            link: Some(format!("{}/{}", uri.clone(), _uri4md)),
            description: None,
            author: None,
            categories: vec![],
            comments: None,
            enclosure: None,
            guid: None,
            pub_date: Some(date),
            source: None,
            content: Some(content),
            ..Default::default()
        };
        channel.items.push(item);
    }
    // Write the RSS XML to the output file
    let mut output_file = File::create(rssfile)?;
    output_file.write_fmt(format_args!("{}", channel))?;

    Ok(())
}

fn site_uri(path: String, base: &str) -> String {
    log::debug!("\n {} ~ {}", path, base);
    let parent_iter = Path::new(&path)
        .ancestors()
        .next()
        .unwrap()
        .strip_prefix(base)
        .unwrap()
        .components()
        .rev();
    let mut uri = String::new();
    for component in parent_iter {
        if let Component::Normal(normal) = component {
            uri.insert_str(0, normal.to_str().unwrap());
            uri.insert(0, '/');
        }
    }
    uri
}

fn is_hidden(entry: &WalkDirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with("."))
        .unwrap_or(false)
}

fn get_directory(path_str: &str) -> Option<String> {
    let path = Path::new(path_str);
    path.parent()
        .map(|parent| parent.to_str().unwrap().to_owned())
}

fn read_file(filename: &str) -> Result<String, std::io::Error> {
    let mut file = match File::open(filename) {
        Ok(f) => f,
        Err(e) => return Err(e),
    };
    let mut contents = String::new();
    match file.read_to_string(&mut contents) {
        Ok(_) => Ok(contents),
        Err(e) => Err(e),
    }
}
