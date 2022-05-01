use anyhow::{Context, Result};
use clap::{ArgEnum, Parser};
use rusqlite::Connection;
use std::cmp::Ordering;
use std::collections::HashMap;

#[derive(Parser, Debug)]
struct Args {
    #[clap(short, long, arg_enum)]
    input_type: InputType,

    #[clap(short, long)]
    file: String,
}

#[derive(ArgEnum, Clone, Debug)]
enum InputType {
    Kobo,
    Oreilly,
}

fn main() {
    let args = Args::parse();

    let texts = match args.input_type {
        InputType::Kobo => parse_kobo(&args.file),
        InputType::Oreilly => parse_oreilly(&args.file),
    }
    .unwrap();

    for text in texts {
        println!("{}", text_to_markdown(&text));
    }
}

fn parse_kobo(db_file_path: &str) -> Result<Vec<Text>> {
    const SQL_QUERY: &str = r#"
SELECT
  c.ISBN AS ISBN,
  ac.Attribution AS author,
  c.BookTitle AS bookTitle,
  c.title AS title,
  bookmark.text AS highlight,
  bookmark.Annotation AS annotation,
  bookmark.StartOffset AS startOffset,
  bookmark.EndOffset AS endOffset,
  bookmark.StartContainerPath AS startContainerPath,
  bookmark.EndContainerPath AS endContainerPath
FROM bookmark
LEFT OUTER JOIN content c
  ON (c.ContentID LIKE bookmark.ContentId || '%' AND c.MimeType LIKE '%epub%')
LEFT OUTER JOIN content ac
  ON (ac.ContentID = bookmark.VolumeID AND ac.ContentType = 6)
WHERE text IS NOT NULL"#;

    let conn = Connection::open(db_file_path)?;

    let mut query = conn
        .prepare(SQL_QUERY)
        .context("Failed to prepare SQL query")?;

    let mut highlights: Vec<KoboHighlightEntry> = query
        .query_map([], |row| {
            Ok(KoboHighlightEntry {
                isbn: row.get(0)?,
                author: row.get(1)?,
                book_title: row.get(2)?,
                title: row.get(3)?,
                highlight: row.get(4)?,
                annotation: row.get(5)?,
                start_offset: row.get(6)?,
                end_offset: row.get(7)?,
                start_container_path: row.get(8)?,
                end_container_path: row.get(9)?,
            })
        })?
        .flatten()
        .collect();

    highlights.sort_by(|a, b| {
        a.book_title
            .cmp(&b.book_title)
            .then(a.start_container_path.cmp(&b.start_container_path))
            .then(a.start_offset.cmp(&b.start_offset))
    });

    let highlights_by_book = highlights.into_iter().fold(HashMap::new(), |mut acc, hl| {
        acc.entry(
            hl.book_title
                .clone()
                .unwrap_or_else(|| "(No title)".to_string()),
        )
        .or_insert(Vec::new())
        .push(hl);
        acc
    });

    let texts = highlights_by_book
        .into_iter()
        .map(|(book_title, highlights)| Text {
            title: book_title,
            author: highlights
                .first()
                .and_then(|hl| hl.author.clone())
                .unwrap_or_else(|| "(Author unknown)".to_string()),
            highlights: highlights.into_iter().map(|hl| hl.highlight).collect(),
        });

    Ok(texts.collect())
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct KoboHighlightEntry {
    isbn: Option<String>,
    author: Option<String>,
    book_title: Option<String>,
    title: String,
    highlight: String,
    annotation: Option<String>,
    start_offset: i32,
    end_offset: i32,
    start_container_path: String,
    end_container_path: String,
}

fn parse_oreilly(json_file_path: &str) -> Result<Vec<Text>> {
    Ok(Default::default())
}

#[derive(Clone, Debug, Default)]
struct Text {
    title: String,
    author: String,
    highlights: Vec<String>,
}

fn text_to_markdown(text: &Text) -> String {
    let mut md = String::new();

    md.push_str(&format!(
        "# {}\n\nAuthor: {}\n\n",
        &text.title, &text.author
    ));

    for hl in text.highlights.iter() {
        md.push_str(
            &hl.lines()
                .map(|line| format!("> {}\n", line.trim()))
                .collect::<String>(),
        );
        md.push('\n');
    }

    md
}
