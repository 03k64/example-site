use actix_files::Files;
use actix_web::{
    get, http::StatusCode, web::Path, App, HttpResponse, HttpServer, Responder, ResponseError,
};
use anyhow::Context;
use askama_actix::{Template, TemplateToResponse};
use serde::Deserialize;
use std::{
    error::Error,
    fmt,
    fmt::{Debug, Formatter},
    fs::File,
    io,
};

type HttpResult = Result<HttpResponse, SiteError>;

#[derive(thiserror::Error)]
enum SiteError {
    #[error("Failed to deserialize slides")]
    InvalidSlidesError(#[source] anyhow::Error),
    #[error(transparent)]
    MissingSlidesError(#[from] anyhow::Error),
}

impl Debug for SiteError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        writeln!(f, "{}\n", self)?;

        let mut current = self.source();
        while let Some(cause) = current {
            writeln!(f, "Caused by:\n\t{}", cause)?;
            current = cause.source();
        }

        Ok(())
    }
}

impl ResponseError for SiteError {
    fn error_response(&self) -> HttpResponse {
        match self {
            Self::InvalidSlidesError(_) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
            Self::MissingSlidesError(_) => HttpResponse::new(StatusCode::NOT_FOUND),
        }
    }
}

#[derive(Default, Template)]
#[template(path = "index.html")]
struct Index {
    show_footer: bool,
    show_header: bool,
    title: String,
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct Slide {
    author: Option<String>,
    content: String,
    date: Option<String>,
    subtitle: Option<String>,
    title: String,
}

#[derive(Template)]
#[template(path = "slides.html")]
struct Slides {
    show_footer: bool,
    show_header: bool,
    slides: Vec<Slide>,
    title: String,
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    HttpServer::new(move || {
        App::new()
            .service(index)
            .service(presentation)
            .service(Files::new("/static", "./static"))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}

#[get("/")]
async fn index() -> impl Responder {
    Index {
        title: "Home".to_owned(),
        ..Default::default()
    }
    .to_response()
}

#[get("/slides/{title}")]
async fn presentation(title: Path<String>) -> HttpResult {
    let path = format!("./slides/{}.yml", &title);

    let file = File::open(&path)
        .context("Failed to read slides")
        .map_err(SiteError::MissingSlidesError)?;

    let slides: Vec<Slide> = serde_yaml::from_reader(&file)
        .context("Failed to deserialize slides")
        .map_err(SiteError::InvalidSlidesError)?;

    let response = Slides {
        show_footer: true,
        show_header: true,
        slides,
        title: "Rust: An Introduction".to_owned(),
    }
    .to_response();

    Ok(response)
}
