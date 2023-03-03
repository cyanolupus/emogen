use worker::*;
use emogen::Emogen;
use rusttype::Font;

mod utils;
mod emogen;

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
    let compressed_ttf = include_bytes_zstd::include_bytes_zstd!("./static/Koruri-Extrabold-subset.ttf", 21);
    let font = Font::try_from_vec(compressed_ttf).unwrap();
    let height = 128;
    let width = 128;
    let generator = Emogen::new(service_name.to_string(), base_domain.to_string(), html.to_string(), font, height, width);

    let router = Router::with_data(generator);
    router
        .get("/", |req, ctx| ctx.data.response_html(req, &ctx))
        .get("/:moji", |req, ctx| ctx.data.response_html(req, &ctx))
        
        .get("/:moji/e.png", |req, ctx| ctx.data.response_emoji_png(req, &ctx))
        .get("/:moji/e.ico", |req, ctx| ctx.data.response_emoji_ico(req, &ctx))
        .get("/:moji/e.jpg", |req, ctx| ctx.data.response_emoji_jpg(req, &ctx))
        .get("/:moji/e.gif", |req, ctx| ctx.data.response_emoji_gif(req, &ctx))

        .get("/:moji/png", |req, ctx| ctx.data.response_emoji_png(req, &ctx))
        .get("/:moji/ico", |req, ctx| ctx.data.response_emoji_ico(req, &ctx))
        .get("/:moji/jpg", |req, ctx| ctx.data.response_emoji_jpg(req, &ctx))
        .get("/:moji/gif", |req, ctx| ctx.data.response_emoji_gif(req, &ctx))

        .run(req, env)
        .await
}