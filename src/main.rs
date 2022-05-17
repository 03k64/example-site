use actix_files::Files;
use actix_web::{
    get, http::StatusCode, middleware::Logger, web, web::Path, App, HttpRequest, HttpResponse,
    HttpServer, Responder, ResponseError,
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
        log::error!("{}", self);

        let mut error_page = ErrorPage {
            show_footer: true,
            show_header: false,
            title: "Error".to_owned(),
            ..Default::default()
        };

        let error_status = match self {
            Self::InvalidSlidesError(e) => {
                error_page.error = format!("{:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            }
            Self::MissingSlidesError(e) => {
                error_page.error = format!("{:?}", e);
                StatusCode::NOT_FOUND
            }
        };

        let mut response = error_page.to_response();
        let status = response.status_mut();
        *status = error_status;

        response
    }
}

#[derive(Default, Template)]
#[template(path = "error.html")]
struct ErrorPage {
    error: String,
    show_footer: bool,
    show_header: bool,
    title: String,
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
    env_logger::init();

    HttpServer::new(move || {
        App::new()
            .service(index)
            .service(presentation)
            .service(Files::new("/static", "./static"))
            .default_service(web::to(not_found))
            .wrap(Logger::default())
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

async fn not_found(request: HttpRequest) -> HttpResponse {
    ErrorPage {
        error: format!("No matching route for {}", request.uri()),
        show_footer: true,
        show_header: false,
        title: "Error".to_owned(),
    }
    .to_response()
}
