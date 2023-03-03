use worker::*;
use image::Rgba;
use urlencoding;
use imageproc::drawing::{draw_text_mut, Canvas};
use rusttype::{Font, Scale};

pub struct Emogen {
    service_name: String,
    base_domain: String,
    html_tmpl: String,
    font: rusttype::Font<'static>,
    height: u32,
    width: u32,
}

impl Emogen {
    pub fn new(service_name: String, base_domain: String, html_tmpl: String, font: rusttype::Font<'static>, height: u32, width: u32) -> Emogen {
        Emogen {
            service_name,
            base_domain,
            html_tmpl,
            font,
            height,
            width,
        }
    }

    pub fn response_html<D>(&self, req: Request, ctx: &RouteContext<D>) -> Result<Response> {
        let host = req.headers().get("host").unwrap_or_default().unwrap_or_default();
        let moji = ctx.param("moji").unwrap_or(&"".to_string()).to_string();
        let moji_decoded = decode_moji(&moji);
        let query = match req.url() {
            Ok(url) => url.query().unwrap_or_default().to_string(),
            Err(_) => "".to_string(),
        };
        if moji == "" {
            Response::from_html(self.html_tmpl
                .replace("{{ .Domain }}", &host)
                .replace("{{ .Moji }}", &self.service_name)
                .replace("{{ .Moji_decoded }}", &self.service_name)
                .replace("{{ .Query }}", &query)
            )
        } else {
            Response::from_html(self.html_tmpl
                .replace("{{ .Domain }}", &host)
                .replace("{{ .Moji }}", &moji)
                .replace("{{ .Moji_decoded }}", &moji_decoded)
                .replace("{{ .Query }}", &query)
            )
        }
    }

    pub fn response_emoji_png<D>(&self, req: Request, ctx: &RouteContext<D>) -> Result<Response> {
        self.response_emoji(req, ctx, image::ImageOutputFormat::Png)
    }

    pub fn response_emoji_ico<D>(&self, req: Request, ctx: &RouteContext<D>) -> Result<Response> {
        self.response_emoji(req, ctx, image::ImageOutputFormat::Ico)
    }

    pub fn response_emoji_jpg<D>(&self, req: Request, ctx: &RouteContext<D>) -> Result<Response> {
        self.response_emoji(req, ctx, image::ImageOutputFormat::Jpeg(100))
    }

    pub fn response_emoji_gif<D>(&self, req: Request, ctx: &RouteContext<D>) -> Result<Response> {
        self.response_emoji(req, ctx, image::ImageOutputFormat::Gif)
    }

    pub fn response_emoji<D>(&self, req: Request, ctx: &RouteContext<D>, format: image::ImageOutputFormat) -> Result<Response> {
        let moji = ctx.param("moji").unwrap_or(&"".to_string()).to_string();
        let moji_decoded = decode_moji(&moji);

        let (fg, bg) = get_color(req, &self.base_domain);
        
        let emoji = self.emoji_generator(moji_decoded.to_string(), fg, bg);
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
            Err(_) => Response::error("Failed to generate emoji", 500)
        }
    }

    fn emoji_generator(&self, moji: String, fg: Rgba<u8>, bg: Rgba<u8>) -> image::DynamicImage {
        let mut img = image::DynamicImage::new_rgba8(self.width, self.height);
    
        let x = 0;
        let mut y = 0;
        let height_f32 = self.height as f32;
        let width_f32 = self.width as f32;
    
        for x in 0..self.width {
            for y in 0..self.height {
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
            let scale_subdomain = get_scale_by_font(height_f32 / moji_lines_len_f32, width_f32, &self.font, &moji_line.to_string());
            draw_text_mut(&mut img, fg, x, y, scale_subdomain, &self.font, &moji_line);
            y += self.height / moji_lines_len;
        }
    
        let scale_subdomain = get_scale_by_font(height_f32, width_f32, &self.font, &moji);
        draw_text_mut(&mut img, fg, x, y, scale_subdomain, &self.font, &moji);
    
        img
    }
}

fn decode_moji(moji: &String) -> String {
    urlencoding::decode(&moji.replace("%0a", "\n").replace("%0A", "\n")).unwrap_or_default().to_string()
}

fn get_color(req: Request, base_domain: &String) -> (Rgba<u8>, Rgba<u8>) {
    let fg_q = url_param(&req, "fg".to_string()).unwrap_or("0a0f".to_string());
    let bg_q = url_param(&req, "bg".to_string()).unwrap_or("0000".to_string());

    let subdomain = req.headers().get("host").unwrap_or_default().unwrap_or_default().to_string().replace(&format!(".{}", base_domain), "");
    if subdomain.len() == 8 {
        (colorcode2color(subdomain[0..4].to_string()), colorcode2color(subdomain[4..8].to_string()))
    } else if subdomain.len() == 16 {
        (colorcode2color(subdomain[0..8].to_string()), colorcode2color(subdomain[8..16].to_string()))
    } else {
        (colorcode2color(fg_q), colorcode2color(bg_q))
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