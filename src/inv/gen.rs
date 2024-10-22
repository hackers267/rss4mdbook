use super::util::pick_field;
use chrono::prelude::*;
use chrono::DateTime;
use rss::Channel;
use rss::Item;
use scraper::{Html, Selector};
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;
use toml::Value;
use walkdir::{DirEntry as WalkDirEntry, WalkDir};

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
    let book_path = Path::new(&book);
    match read_file(book_path) {
        Ok(contents) => {
            let toml_value = contents.parse::<Value>().unwrap();
            let src = pick_src(&toml_value);
            let output = pick_field(&toml_value, "build", "build-dir").unwrap_or("book");
            match rss_base_url(&toml_value) {
                Some(rss_url_base) => {
                    // url-base 存在，并且是字符串类型
                    println!("Found url-base: {}", rss_url_base);
                    let rss_title = pick_rss_title(&toml_value);
                    let rss_desc = pick_rss_desc(&toml_value);
                    let src_path = source_path(book_path, src);
                    let output_path = output_path(book_path, output);
                    let export_rss_path = rss_output_path(book_path);
                    if let (Some(source_path), Some(exp_rss_path), Some(output_path)) =
                        (src_path, export_rss_path, output_path)
                    {
                        let latest5files = scan_dir(&source_path, 4);
                        println!("will export these article into RSS.xml");
                        let rss_config = RssConfig::new(rss_title, rss_desc, rss_url_base);
                        match rss4top5md(
                            &exp_rss_path,
                            &source_path,
                            &latest5files,
                            &output_path,
                            &rss_config,
                        ) {
                            Ok(_) => println!("\n Export => {:?}\n\n", exp_rss_path.clone()),
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

/// 获取rss输出地址
fn rss_output_path(book_path: &Path) -> Option<PathBuf> {
    book_path.parent().map(|path| path.join("book/RSS.xml"))
}

/// 获取源码地址
fn source_path(book_path: &Path, src: &str) -> Option<PathBuf> {
    book_path.parent().map(|path| path.join(src))
}

/// 获取源码地址
fn output_path(book_path: &Path, output: &str) -> Option<PathBuf> {
    book_path.parent().map(|path| path.join(output))
}

/// 提取RSS描述
fn pick_rss_desc(toml_value: &Value) -> &str {
    pick_field(toml_value, "rss4mdbook", "rss_desc").unwrap_or("Welcome To Subscribe")
}

/// 提取RSS标题
fn pick_rss_title(toml_value: &Value) -> &str {
    pick_field(toml_value, "rss4mdbook", "rss_desc").unwrap_or("Thanks Subscribe")
}

/// 提取rss输出中的base-url字段
fn rss_base_url(toml_value: &Value) -> Option<&str> {
    pick_field(toml_value, "rss4mdbook", "url-base")
}

/// 获取输入目录
fn pick_src(toml_value: &Value) -> &str {
    pick_field(toml_value, "book", "src").unwrap_or("src")
}

fn scan_dir(source: &Path, top_n: usize) -> Vec<PathBuf> {
    let walker = WalkDir::new(source).into_iter();
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
    file_modified_times
        .iter()
        .rev()
        .filter(|(path, _)| !path.to_string_lossy().contains("SUMMARY.md"))
        .take(top_n)
        .map(|(path, _)| path.to_path_buf())
        .collect()
}

struct RssConfig<'a> {
    title: &'a str,
    desc: &'a str,
    url: &'a str,
}

impl<'a> RssConfig<'a> {
    pub fn new(title: &'a str, desc: &'a str, url: &'a str) -> Self {
        Self { title, desc, url }
    }
}

fn rss4top5md(
    rssfile: &Path,
    source: &Path,
    latest5files: &[PathBuf],
    output_path: &Path,
    rss_config: &RssConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    // 创建一个 RSS channel
    let uri = rss_config.url;
    let title = rss_config.title;
    let desc = rss_config.desc;
    let mut channel = Channel {
        link: uri.to_string(),
        title: title.to_string(),
        description: desc.to_string(),
        generator: Some("my_rss_generator".to_owned()),
        ..Default::default()
    };

    // 为每个文件创建 RSS item
    for file in latest5files {
        let _p4src = site_uri(file, source);
        let _uri4md = &_p4src[.._p4src.len() - 3];
        let metadata = fs::metadata(file)?;
        let date = DateTime::<Local>::from(metadata.modified()?).to_rfc2822();
        let file_path = PathBuf::from(&file);
        let content = pick_content(&file_path, source, output_path)?;
        let file_name = file_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let item = Item {
            title: Some(file_name),
            link: Some(format!("{}/{}", uri, _uri4md)),
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

/// 读取内容
fn pick_content(file: &Path, source: &Path, output: &Path) -> Result<String, Box<dyn Error>> {
    let file_path = file.strip_prefix(source).unwrap();
    let mut file_path = output.join(file_path);
    file_path.set_extension("html");
    let prefix = &file_path
        .as_path()
        .file_stem()
        .and_then(|v| v.to_str())
        .map(|v| v.to_lowercase());
    if prefix.as_ref().is_some_and(|v| v == "readme") {
        file_path.set_file_name("index.html")
    }
    let content = fs::read_to_string(&file_path)?;
    let html = Html::parse_document(&content);
    let main_selector = Selector::parse("main").unwrap();
    let main = html.select(&main_selector).next().unwrap().inner_html();
    Ok(main)
}

fn site_uri(path: &Path, base: &Path) -> String {
    let parent_iter = path
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

fn read_file(filename: &Path) -> Result<String, std::io::Error> {
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
