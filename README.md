# RSS for mdBook
> a generator for mdBook as CLI tool, export RSS.xml into u want path...

------
## background

mdBook is great, but not auto export RSS,
and the mdbook-rss is not work now...

so make it self ;-)


------
### goal
> as Rustacean homework ...

as crate, can:

- easy install
- usage at local
- usage after mdBook generated static site, 
    - scanning .md path, 
    - generat RSS.xml into export path
    - ...so we hold lasted upgrade content's RSS

------
## Installation

### Cargo
If you already have a Rust environment set up, you can use the cargo install command:

> $ cargo install --git https://github.com/hackers267/rss4mdbook.git

Cargo will build the `rss4mdbook` binary and place it in $HOME/.cargo.


### Manual installation from GitHub

Compiled binary versions of `rss4mdbook` are uploaded to GitHub when a release is made. You can install `rss4mdbook` manually by downloading a release, extracting it, and copying the binary to a directory in your `$PATH`, such as `/usr/local/bin`.

## Usage
> daily usage , only one shot:

- 0: config mdBook's book.toml, append such as:

```toml
...
[rss4mdbook]
url-base = "https://example.com"
```

- 1: mdbook build
- 2: use `gen` command, append the lasted 4 articles as rss.xml 

```
$ rss4mdbook gen /path/2u/mdbook/book.toml
```

