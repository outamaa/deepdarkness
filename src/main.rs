use anyhow::{Context, Result};
use clap::{ArgEnum, Parser};
use rusqlite::Connection;

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

    let text = match args.input_type {
        InputType::Kobo => parse_kobo(&args.file),
        InputType::Oreilly => parse_oreilly(&args.file),
    };
    println!("{:?}", text);
}

fn parse_kobo(db_file_path: &str) -> Result<Text> {
    const SQL_QUERY: &str = r#"
select
  content.ISBN as ISBN,
  content.BookTitle as bookTitle,
  content.title as title,
  bookmark.text as highlight,
  bookmark.Annotation as annotation,
  bookmark.StartOffset as startOffset,
  bookmark.EndOffset as endOffset,
  bookmark.StartContainerPath as startContainerPath,
  bookmark.EndContainerPath as endContainerPath
from bookmark
left outer join content
on (content.ContentID LIKE bookmark.ContentId || '%'  AND content.MimeType LIKE '%epub%')
where text is not null
order by bookTitle"#;
    let conn = Connection::open(db_file_path)?;

    let mut query = conn
        .prepare(SQL_QUERY)
        .context("Failed to prepare SQL query")?;

    let highlights = query.query_map([], |row| {
        Ok(KoboHighlightEntry {
            isbn: row.get(0)?,
            book_title: row.get(1)?,
            title: row.get(2)?,
            highlight: row.get(3)?,
            annotation: row.get(4)?,
            start_offset: row.get(5)?,
            end_offset: row.get(6)?,
            start_container_path: row.get(7)?,
            end_container_path: row.get(8)?,
        })
    })?;

    for h in highlights {
        match h {
            Ok(highlight) => {
                println!("{:?}", highlight);
            }
            Err(e) => {
                println!("{:?}", e);
            }
        }
    }

    Ok(Default::default())
}

#[derive(Clone, Debug, Default)]
struct KoboHighlightEntry {
    isbn: Option<String>,
    book_title: Option<String>,
    title: String,
    highlight: String,
    annotation: Option<String>,
    start_offset: i32,
    end_offset: i32,
    start_container_path: String,
    end_container_path: String,
}

fn parse_oreilly(json_file_path: &str) -> Result<Text> {
    Ok(Default::default())
}

#[derive(Clone, Debug, Default)]
struct Text {
    title: String,
    author: String,
    highlights: Vec<Highlight>,
}

#[derive(Clone, Debug, Default)]
struct Highlight {
    text: String,
    pos: usize,
}
