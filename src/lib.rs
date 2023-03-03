use worker::*;
use image::Rgba;
use imageproc::drawing::{draw_text_mut, Canvas};
use rusttype::{Font, Scale};
use urlencoding::decode;

mod utils;

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

    let router = Router::new();
    router
        .get("/", |req, _| {
            let host = req.headers().get("host").unwrap_or_default();
            match host {
                Some(host) => {
                    let html = create_html(host);
                    Response::from_html(html)
                }
                None => Response::ok(""),
            }
        })

        .get("/:moji", |req, ctx| {
            let host = req.headers().get("host").unwrap_or_default();
            let url_result = req.url();
            let query = match url_result {
                Ok(url) => get_color_query_from_url(url),
                Err(_) => "".to_string(),
            };
            
            match host {
                Some(host) => {
                    if let Some(moji) = ctx.param("moji") {
                        match decode(&moji.replace("%0a", "\n").replace("%0A", "\n")) {
                            Ok(moji_decoded) => {
                                let html = create_html_with_emoji(host, moji.to_string(), moji_decoded.to_string(), query);
                                Response::from_html(html)
                            }
                            Err(_) => {
                                Response::error("Failed to decode moji", 500)
                            }
                        }
                    } else {
                        Response::error("Not found", 404)
                    }
                }
                None => Response::ok(""),
            }
        })
        .get("/:moji/e.png", |req, ctx| response_emoji(req, ctx, image::ImageOutputFormat::Png))
        .get("/:moji/e.ico", |req, ctx| response_emoji(req, ctx, image::ImageOutputFormat::Ico))
        .get("/:moji/e.jpg", |req, ctx| response_emoji(req, ctx, image::ImageOutputFormat::Jpeg(100)))
        .get("/:moji/e.gif", |req, ctx| response_emoji(req, ctx, image::ImageOutputFormat::Gif))
        .get("/:moji/png", |req, ctx| response_emoji(req, ctx, image::ImageOutputFormat::Png))
        .get("/:moji/ico", |req, ctx| response_emoji(req, ctx, image::ImageOutputFormat::Ico))
        .get("/:moji/jpg", |req, ctx| response_emoji(req, ctx, image::ImageOutputFormat::Jpeg(100)))
        .get("/:moji/gif", |req, ctx| response_emoji(req, ctx, image::ImageOutputFormat::Gif))

        .run(req, env)
        .await
}

fn get_color_from_url(url: Url) -> (Rgba<u8>, Rgba<u8>) {
    let fg_q = url.query_pairs().find(|(key, _)| key == "fg");
    let bg_q = url.query_pairs().find(|(key, _)| key == "bg");
    match (fg_q, bg_q) {
        (Some((_, fg)), Some((_, bg))) => {
            (text2color(fg.to_string()), text2color(bg.to_string()))
        }
        (Some((_, fg)), None) => {
            (text2color(fg.to_string()), Rgba([0, 0, 0, 0]))
        }
        (None, Some((_, bg))) => {
            (Rgba([0, 128, 0, 255]), text2color(bg.to_string()))
        }
        (None, None) => (Rgba([0, 128, 0, 255]), Rgba([0, 0, 0, 0])),
    }
}

fn get_color_query_from_url(url: Url) -> String {
    let fg_q = url.query_pairs().find(|(key, _)| key == "fg");
    let bg_q = url.query_pairs().find(|(key, _)| key == "bg");
    match (fg_q, bg_q) {
        (Some((_, fg)), Some((_, bg))) => {
            format!("?fg={}&bg={}", fg, bg)
        }
        (Some((_, fg)), None) => {
            format!("?fg={}", fg)
        }
        (None, Some((_, bg))) => {
            format!("?bg={}", bg)
        }
        (None, None) => "".to_string(),
    }
}

fn get_color_from_subdomain(subdomain: String) -> (Rgba<u8>, Rgba<u8>) {
    let fg = text2color(subdomain[0..4].to_string());
    let bg = text2color(subdomain[4..8].to_string());
    (fg, bg)
}

fn parse_host(host: String) -> (String, String) {
    let mut subdomain = String::new();
    let domain = host;
    if domain.contains(".urem.uk") {
        subdomain = domain.replace(".urem.uk", "");
    }
    (subdomain, domain)
}

fn response_emoji(req: Request, ctx: RouteContext<()>, format: image::ImageOutputFormat) -> std::result::Result<worker::Response, worker::Error> {
    let url_result = req.url();
    let (mut fg, mut bg) = match url_result {
        Ok(url) => get_color_from_url(url),
        Err(_) => (Rgba([0, 0, 0, 255]), Rgba([255, 255, 255, 255])),
    };
    let host = req.headers().get("host").unwrap_or_default();
    (fg, bg) = match host {
        Some(host) => {
            let (subdomain, _) = parse_host(host.to_string());
            if subdomain.len() == 8 {
                get_color_from_subdomain(subdomain)
            } else {
                (fg, bg)
            }
        }
        None => (fg, bg),
    };
    if let Some(moji) = ctx.param("moji") {
        match decode(&moji.replace("%0a", "\n").replace("%0A", "\n")) {
            Ok(moji_decoded) => {
                let emoji = emoji_generator(moji_decoded.to_string(), fg, bg);
                let mut emoji_img = Vec::new();
                let mime = format2mime(&format);
                match emoji.write_to(&mut emoji_img, format) {
                    Ok(_) => {
                        let mut headers = Headers::new();
                        match headers.set("Content-Type", &mime) {
                            Ok(_) => {
                                let response = Response::from_bytes(emoji_img)?;
                                Ok(response.with_headers(headers))
                            }
                            Err(_) => {
                                Response::error("Failed to set header", 500)
                            }
                        }
                        
                    }
                    Err(_) => Response::ok(""),
                }
            }
            Err(_) => {
                Response::error("Failed to decode moji", 500)
            }
        }
    } else {
        Response::error("Not found", 404)
    }
}

fn format2mime(format: &image::ImageOutputFormat) -> String {
    match format {
        image::ImageOutputFormat::Png => "image/png".to_string(),
        image::ImageOutputFormat::Jpeg(_) => "image/jpeg".to_string(),
        image::ImageOutputFormat::Gif => "image/gif".to_string(),
        _ => "".to_string(),
    }
}

fn emoji_generator(moji: String, fg: Rgba<u8>, bg: Rgba<u8>) -> image::DynamicImage {
    let height = 512;
    let width = 512;
    let compressed_ttf = include_bytes_zstd::include_bytes_zstd!("./static/Koruri-Extrabold-subset.ttf", 21);
    let font = Font::try_from_vec(compressed_ttf).unwrap();

    let mut img = image::DynamicImage::new_rgba8(width, height);

    let x = 0;
    let mut y = 0;
    let height_f32 = height as f32;
    let width_f32 = width as f32;

    for x in 0..width {
        for y in 0..height {
            img.draw_pixel(x, y, bg)
        }
    }

    let moji_lines = moji.split("\n");
    let moji_lines_len = moji_lines.clone().count() as u32;
    let moji_lines_len_f32 = moji_lines_len as f32;
    if moji_lines_len == 0 {
        return img;
    }
    for moji_line in moji_lines {
        let scale_subdomain = get_scale_by_font(height_f32 / moji_lines_len_f32, width_f32, &font, &moji_line.to_string());
        draw_text_mut(&mut img, fg, x, y, scale_subdomain, &font, &moji_line);
        y += height / moji_lines_len;
    }

    let scale_subdomain = get_scale_by_font(height_f32, width_f32, &font, &moji);
    draw_text_mut(&mut img, fg, x, y, scale_subdomain, &font, &moji);

    img
}

fn get_scale_by_font(height: f32, width: f32, font: &Font, text: &String) -> Scale {
    let mut glyph_width_sum = 0.0;
    for c in text.chars() {
        let glyph = font.glyph(c).scaled(Scale::uniform(height));
        glyph_width_sum += glyph.h_metrics().advance_width;
    }
    if glyph_width_sum == 0.0 {
        glyph_width_sum = 1.0;
    }
    let scale = Scale {
        x: height * width / glyph_width_sum,
        y: height,
    };
    scale
}

fn create_html(domain: String) -> String {
    let html = include_str!("../static/index.html.tmpl");
    html.replace("{{ .Domain }}", &domain)
}

fn create_html_with_emoji(domain: String, moji: String, moji_decoded: String, query: String) -> String {
    let html = include_str!("../static/emojipreview.html.tmpl");
    html.replace("{{ .Domain }}", &domain)
        .replace("{{ .Moji }}", &moji)
        .replace("{{ .Moji_decoded }}", &moji_decoded)
        .replace("{{ .Query }}", &query)
}

fn text2color(text: String) -> Rgba<u8> {
    let mut color = Rgba([0, 0, 0, 255]);
    if text.len() == 4 {
        let r = u8::from_str_radix(&text[0..1], 16).unwrap_or(0) * 16;
        let g = u8::from_str_radix(&text[1..2], 16).unwrap_or(0) * 16;
        let b = u8::from_str_radix(&text[2..3], 16).unwrap_or(0) * 16;
        let a = u8::from_str_radix(&text[3..4], 16).unwrap_or(255) * 16;
        color = Rgba([r, g, b, a]);
    }
    color
}