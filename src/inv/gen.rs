use super::util::pick_field;
use chrono::{DateTime, Local};
use log::{error, info, warn};
use rss::{Channel, Item};
use scraper::{Html, Selector};
use std::{
    error::Error,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use toml::Value;
use walkdir::{DirEntry as WalkDirEntry, WalkDir};

/* CLI for gen. RSS from mdBook
    - walk the src path
    - check all .md file's update date
    - order pick lated 5
    - export as rss.xml -> u want path
*/
pub fn exp(book: String, limit: Option<usize>, day: Option<usize>) {
    let pkg_name = option_env!("CARGO_PKG_NAME").unwrap_or("DAMA's Crate");
    let pkg_version = option_env!("CARGO_PKG_VERSION").unwrap_or("0.1.42");
    let top = limit.unwrap_or(5);
    info!(
        "digging and generating by\n\t~> {} v{} <~",
        pkg_name, pkg_version
    );
    info!("let's make RSS now...");
    let book_path = Path::new(&book);
    match read_file(book_path) {
        Ok(contents) => {
            let toml_value = contents.parse::<Value>().unwrap();
            let src = pick_src(&toml_value);
            let output = pick_field(&toml_value, "build", "build-dir").unwrap_or("book");
            let author = pick_author(&toml_value).unwrap_or("unknown");
            match rss_base_url(&toml_value) {
                Some(rss_url_base) => {
                    // url-base 存在，并且是字符串类型
                    info!("Found url-base: {}", rss_url_base);
                    let rss_title = pick_rss_title(&toml_value);
                    let rss_desc = pick_rss_desc(&toml_value);
                    let src_path = source_path(book_path, src);
                    let output_path = output_path(book_path, output);
                    let export_rss_path = rss_output_path(book_path);
                    if let (Some(source_path), Some(exp_rss_path), Some(output_path)) =
                        (src_path, export_rss_path, output_path)
                    {
                        let latest_files = scan_dir(&source_path, top, day);
                        info!("Will export these article into RSS.xml");
                        latest_files
                            .iter()
                            .for_each(|path| info!("OUTPUT {}", path.to_str().unwrap_or_default()));
                        let rss_config = RssConfig::new(rss_title, rss_desc, rss_url_base, author);
                        match rss_top(
                            &exp_rss_path,
                            &source_path,
                            &latest_files,
                            &output_path,
                            &rss_config,
                        ) {
                            Ok(_) => info!("\n Export => {:?}\n\n", exp_rss_path.clone()),
                            Err(e) => error!("Error: {}", e),
                        }
                    }
                }
                None => {
                    // url-base 不存在或不是字符串类型
                    warn!(
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
        Err(e) => error!("Error: {}", e),
    }
}

fn pick_author(toml_value: &Value) -> Option<&str> {
    toml_value
        .get("book")
        .and_then(|v| v.get("authors"))
        .and_then(|v| v.as_array())
        .and_then(|array| array.first())
        .and_then(|v| v.as_str())
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
    pick_field(toml_value, "rss4mdbook", "rss_title").unwrap_or("Thanks Subscribe")
}

/// 提取rss输出中的base-url字段
fn rss_base_url(toml_value: &Value) -> Option<&str> {
    pick_field(toml_value, "rss4mdbook", "url-base")
}

/// 获取输入目录
fn pick_src(toml_value: &Value) -> &str {
    pick_field(toml_value, "book", "src").unwrap_or("src")
}

fn scan_dir(source: &Path, top_n: usize, day: Option<usize>) -> Vec<PathBuf> {
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

    match day {
        Some(day) => file_modified_times
            .iter()
            .rev()
            .filter(|(path, _)| {
                !path
                    .to_str()
                    .is_some_and(|v| v.to_lowercase().contains("summary.md"))
            })
            .filter(|(_, time)| latest_days(time, day))
            .map(|(path, _)| path.to_path_buf())
            .collect(),
        None => {
            // 获取最新的n个文件，过滤掉包含 SUMMARY.md 的路径
            file_modified_times
                .iter()
                .rev()
                .filter(|(path, _)| !path.to_string_lossy().contains("SUMMARY.md"))
                .take(top_n)
                .map(|(path, _)| path.to_path_buf())
                .collect()
        }
    }
}

/// 过滤最近几天的内容
fn latest_days(time: &std::time::SystemTime, day: usize) -> bool {
    let modified_time: DateTime<Local> = DateTime::from(*time);
    let now = Local::now();
    let duration = now - modified_time;
    let days = duration.num_days() as usize;
    days < day
}

struct RssConfig<'a> {
    title: &'a str,
    desc: &'a str,
    url: &'a str,
    author: &'a str,
}

impl<'a> RssConfig<'a> {
    pub fn new(title: &'a str, desc: &'a str, url: &'a str, author: &'a str) -> Self {
        Self {
            title,
            desc,
            url,
            author,
        }
    }
}

fn rss_top(
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
        language: Some("chinese".to_string()),
        ..Default::default()
    };
    let author = rss_config.author;

    // 为每个文件创建 RSS item
    for file in latest5files {
        let uri = site_uri(file, source);
        let metadata = fs::metadata(file)?;
        let date = DateTime::<Local>::from(metadata.modified()?).to_rfc2822();
        let file_path = PathBuf::from(&file);
        let content = pick_content(&file_path, source, output_path)?;
        let title = pick_item_title(&file_path, source, output_path)?;
        let item = Item {
            title: Some(title),
            link: Some(uri),
            description: None,
            author: Some(author.to_string()),
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
///
/// # Arguments
/// - file: 文件路径
/// - source: mdbook的源目录路径
/// - output: mdbook的输出目录路径
///
/// # Return
/// 成功则为输出的文件中html中的main标签内容，失败则输出为错误
fn pick_content(file: &Path, source: &Path, output: &Path) -> Result<String, Box<dyn Error>> {
    let content = pick_target_content(file, source, output)?;
    let html = Html::parse_document(&content);
    let main_selector = Selector::parse("main").unwrap();
    let main = html.select(&main_selector).next().unwrap().inner_html();
    Ok(main)
}

/// 获取目标文件的内容
///
/// # Arguments
/// - file: 文件路径
/// - source: mdbook的源目录路径
/// - output: mdbook的输出目录路径
///
/// # Return
/// 成功则为输出的文件内容，失败则输出为错误
fn pick_target_content(
    file: &Path,
    source: &Path,
    output: &Path,
) -> Result<String, Box<dyn Error>> {
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
    Ok(content)
}
/// 读取标题
///
/// # Arguments
/// - file: 文件路径
/// - source: mdbook的源目录路径
/// - output: mdbook的输出目录路径
///
/// # Return
/// 成功则为输出的文件中html中的title属性内容，失败则输出为错误
fn pick_item_title(file: &Path, source: &Path, output: &Path) -> Result<String, Box<dyn Error>> {
    let content = pick_target_content(file, source, output)?;
    let html = Html::parse_document(&content);
    let title_selector = Selector::parse("title")?;
    let title = html.select(&title_selector).next().unwrap().inner_html();
    Ok(title)
}

fn site_uri<'a>(path: &'a Path, base: &'a Path) -> String {
    let uri = path.strip_prefix(base).expect("提取条目的uri失败");
    uri.with_extension("")
        .to_str()
        .map(|v| v.to_string())
        .expect("提取uri条目失败")
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
