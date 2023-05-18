use worker::*;
use emogen::Emogen;

mod utils;
mod emogen;
mod r2;

fn log_request(req: &Request) {
    console_log!(
        "{} - [{}], located at: {:?}, within: {}",
        Date::now().to_string(),
        req.path(),
        req.cf().coordinates().unwrap_or_default(),
        req.cf().region().unwrap_or("unknown region".into())
    );
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    log_request(&req);
    utils::set_panic_hook();

    let service_name = "えもじぇん";
    let base_domain = "urem.uk";
    let html = include_str!("../static/index.html.tmpl");
    let height = 128;
    let width = 128;
    let generator = Emogen::new(service_name.to_string(), base_domain.to_string(), html.to_string(), height, width);

    let router = Router::with_data(generator);
    router
        .get("/", |req, ctx| emogen::response_html(req, &ctx))
        .get("/:moji", |req, ctx| emogen::response_html(req, &ctx))
        
        .get_async("/:moji/e.png", |req, ctx| emogen::response_emoji(req, ctx, image::ImageOutputFormat::Png))
        .get_async("/:moji/e.ico", |req, ctx| emogen::response_emoji(req, ctx, image::ImageOutputFormat::Ico))
        .get_async("/:moji/e.jpg", |req, ctx| emogen::response_emoji(req, ctx, image::ImageOutputFormat::Jpeg(100)))
        .get_async("/:moji/e.gif", |req, ctx| emogen::response_emoji(req, ctx, image::ImageOutputFormat::Gif))

        .get_async("/:moji/png", |req, ctx| emogen::response_emoji(req, ctx, image::ImageOutputFormat::Png))
        .get_async("/:moji/ico", |req, ctx| emogen::response_emoji(req, ctx, image::ImageOutputFormat::Ico))
        .get_async("/:moji/jpg", |req, ctx| emogen::response_emoji(req, ctx, image::ImageOutputFormat::Jpeg(100)))
        .get_async("/:moji/gif", |req, ctx| emogen::response_emoji(req, ctx, image::ImageOutputFormat::Gif))

        .run(req, env)
        .await
}