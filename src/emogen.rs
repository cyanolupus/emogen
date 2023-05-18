use worker::*;
use image::Rgba;
use urlencoding;
use imageproc::drawing::{draw_text_mut, Canvas};
use rusttype::{Font, Scale};
use crate::r2;
pub struct Emogen {
    service_name: String,
    base_domain: String,
    html_tmpl: String,
    height: u32,
    width: u32,
}

impl Emogen {
    pub fn new(service_name: String, base_domain: String, html_tmpl: String, height: u32, width: u32) -> Emogen {
        Emogen {
            service_name,
            base_domain,
            html_tmpl,
            height,
            width,
        }
    }
}

impl Clone for Emogen {
    fn clone(&self) -> Self {
        Emogen {
            service_name: self.service_name.clone(),
            base_domain: self.base_domain.clone(),
            html_tmpl: self.html_tmpl.clone(),
            height: self.height.clone(),
            width: self.width.clone(),
        }
    }
}

pub fn response_html(req: Request, ctx: &RouteContext<Emogen>) -> Result<Response> {
    let data = ctx.data.clone();
    let host = req.headers().get("host").unwrap_or_default().unwrap_or_default();
    let moji = ctx.param("moji").unwrap_or(&"".to_string()).to_string();
    let moji_decoded = decode_moji(&moji);
    let query = match req.url() {
        Ok(url) => url.query().unwrap_or_default().to_string(),
        Err(_) => "".to_string(),
    };
    if moji == "" {
        Response::from_html(data.html_tmpl
            .replace("{{ .Domain }}", &host)
            .replace("{{ .Moji }}", &data.service_name)
            .replace("{{ .Moji_decoded }}", &data.service_name)
            .replace("{{ .Query }}", &query)
        )
    } else {
        Response::from_html(data.html_tmpl
            .replace("{{ .Domain }}", &host)
            .replace("{{ .Moji }}", &moji)
            .replace("{{ .Moji_decoded }}", &moji_decoded)
            .replace("{{ .Query }}", &query)
        )
    }
}

pub async fn response_emoji(req: Request, ctx: RouteContext<Emogen>, format: image::ImageOutputFormat) -> Result<Response> {
    let data = ctx.data.clone();
    let moji = ctx.param("moji").unwrap_or(&"".to_string()).to_string();
    let moji_decoded = decode_moji(&moji);
    let filename = format!("{}.{}", moji_decoded.replace("\n", ""), format2ext(&format));
    let content_disposition = format!("attachment; filename=\"{}\"", filename);

    let (fg, bg) = get_color(req, &data.base_domain);
    
    let emoji = match emoji_generator(moji_decoded.to_string(), fg, bg, ctx).await {
        Some(emoji) => emoji,
        None => return Response::error("Failed to generate emoji", 500)
    };
    let mut emoji_img = Vec::new();
    let mime = format2mime(&format);
    match emoji.write_to(&mut emoji_img, format) {
        Ok(_) => {
            let mut headers = Headers::new();
            let response = Response::from_bytes(emoji_img)?;

            if headers.set("Content-Type", &mime).is_err() {
                return Response::error("Failed to set header", 500);
            }

            if headers.set("Content-Disposition", &content_disposition).is_err() {
                return Response::error("Failed to set header", 500);
            }
            
            Ok(response.with_headers(headers))
        }
        Err(_) => Response::error("Failed to generate emoji", 500)
    }
}

async fn emoji_generator(moji: String, fg: Rgba<u8>, bg: Rgba<u8>, ctx: RouteContext<Emogen>) -> Option<image::DynamicImage> {
    let data = ctx.data.clone();
    let mut img = image::DynamicImage::new_rgba8(data.width, data.height);

    let x = 0;
    let mut y = 0;
    let height_f32 = data.height as f32;
    let width_f32 = data.width as f32;

    let font = match r2::get(ctx, "Koruri-Extrabold.ttf").await {
        Some(font_bytes) => Font::try_from_vec(font_bytes).unwrap(),
        None => return None,
    };

    for x in 0..data.width {
        for y in 0..data.height {
            img.draw_pixel(x, y, bg)
        }
    }

    let moji_lines = moji.split("\n");
    let moji_lines_len = moji_lines.clone().count() as u32;
    let moji_lines_len_f32 = moji_lines_len as f32;
    if moji_lines_len == 0 {
        return Some(img);
    }
    for moji_line in moji_lines {
        let scale_subdomain = get_scale_by_font(height_f32 / moji_lines_len_f32, width_f32, &font, &moji_line.to_string());
        draw_text_mut(&mut img, fg, x, y, scale_subdomain, &font, &moji_line);
        y += data.height / moji_lines_len;
    }

    let scale_subdomain = get_scale_by_font(height_f32, width_f32, &font, &moji);
    draw_text_mut(&mut img, fg, x, y, scale_subdomain, &font, &moji);

    Some(img)
}

fn decode_moji(moji: &String) -> String {
    urlencoding::decode(&moji.replace("%0a", "\n").replace("%0A", "\n")).unwrap_or_default().to_string()
}

fn get_color(req: Request, base_domain: &String) -> (Rgba<u8>, Rgba<u8>) {
    let fg_q = url_param(&req, "fg".to_string());
    let bg_q = url_param(&req, "bg".to_string());

    let subdomain = req.headers().get("host").unwrap_or_default().unwrap_or_default().to_string().replace(&format!(".{}", base_domain), "");
    match (subdomain.len(), fg_q, bg_q) {
        (8, _, _) => (colorcode2color(subdomain[0..4].to_string()), colorcode2color(subdomain[4..8].to_string())),
        (16, _, _) => (colorcode2color(subdomain[0..8].to_string()), colorcode2color(subdomain[8..16].to_string())),
        (_, Some(fg_s), Some(bg_s)) => (colorcode2color(fg_s), colorcode2color(bg_s)),
        (_, Some(fg_s), _) => (colorcode2color(fg_s), get_random_color()),
        (_, _, Some(bg_s)) => (get_random_color(), colorcode2color(bg_s)),
        _ => (get_random_color(), get_random_color())
    }
}

fn url_param(req: &Request, param: String) -> Option<String> {
    match req.url() {
        Ok(url) => {
            match url.query_pairs().find(|(key, _)| key.to_string() == param) {
                Some((_, value)) => Some(value.to_string()),
                None => None,
            }
        }
        Err(_) => None,
    }
}

fn colorcode2color(colorcode: String) -> Rgba<u8> {
    if colorcode.len() == 4 {
        let r = u8::from_str_radix(&colorcode[0..1], 16).unwrap_or_default()*16;
        let g = u8::from_str_radix(&colorcode[1..2], 16).unwrap_or_default()*16;
        let b = u8::from_str_radix(&colorcode[2..3], 16).unwrap_or_default()*16;
        let a = u8::from_str_radix(&colorcode[3..4], 16).unwrap_or_default()*16;
        Rgba([r, g, b, a])
    } else if colorcode.len() == 8 {
        let r = u8::from_str_radix(&colorcode[0..2], 16).unwrap_or_default();
        let g = u8::from_str_radix(&colorcode[2..4], 16).unwrap_or_default();
        let b = u8::from_str_radix(&colorcode[4..6], 16).unwrap_or_default();
        let a = u8::from_str_radix(&colorcode[6..8], 16).unwrap_or_default();
        Rgba([r, g, b, a])
    } else {
        Rgba([0, 0, 0, 0])
    }
}

fn format2mime(format: &image::ImageOutputFormat) -> String {
    match format {
        image::ImageOutputFormat::Png => "image/png".to_string(),
        image::ImageOutputFormat::Jpeg(_) => "image/jpeg".to_string(),
        image::ImageOutputFormat::Gif => "image/gif".to_string(),
        image::ImageOutputFormat::Ico => "image/x-icon".to_string(),
        _ => "".to_string(),
    }
}

fn format2ext(format: &image::ImageOutputFormat) -> String {
    match format {
        image::ImageOutputFormat::Png => "png".to_string(),
        image::ImageOutputFormat::Jpeg(_) => "jpg".to_string(),
        image::ImageOutputFormat::Gif => "gif".to_string(),
        image::ImageOutputFormat::Ico => "ico".to_string(),
        _ => "".to_string(),
    }
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

fn get_random_color() -> Rgba<u8> {
    let rand1 = Box::into_raw(Box::new(42)) as u64;
    let rand2 = Box::into_raw(Box::new(rand1)) as u64;
    let rand3 = Box::into_raw(Box::new(rand2)) as u64;
    let rand4 = Box::into_raw(Box::new(rand3)) as u64;

    let r = ((rand1 + rand2 / rand3 * rand4) % 255) as u8;
    let g = ((rand2 + rand3 / rand4 * rand1) % 255) as u8;
    let b = ((rand3 + rand4 / rand1 * rand2) % 255) as u8;
    let a = ((rand4 + rand1 / rand2 * rand3) % 255) as u8;

    Rgba([r, g, b, a])
}