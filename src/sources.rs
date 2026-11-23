//! Keyless/public source adapters. Ratings are checked again before any thumbnail is fetched.
use crate::{
    model::{Config, Rating, Wallpaper},
    providers::{Network, SearchBatch, check, valid_url},
};
use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::sync::atomic::AtomicBool;
use url::Url;
fn strip_html(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut in_tag = false;
    for ch in raw.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
fn parse_yyyymmdd(s: &str) -> i64 {
    if s.len() != 8 || !s.bytes().all(|b| b.is_ascii_digit()) {
        return 0;
    }
    crate::providers::parse_date(&format!("{}-{}-{}T00:00:00Z", &s[0..4], &s[4..6], &s[6..8]))
}
fn txt(v: &Value, k: &str) -> String {
    v[k].as_str().unwrap_or("").chars().take(4096).collect()
}
fn dim(v: &Value, k: &str) -> u32 {
    crate::model::dimension(&v[k])
}
fn strings(v: &Value) -> Option<Vec<String>> {
    v.as_array()?
        .iter()
        .take(512)
        .map(|x| x.as_str().map(str::to_owned))
        .collect()
}
fn image_url(u: &str) -> bool {
    valid_url(u).is_ok()
        && ["jpg", "jpeg", "png", "webp"].iter().any(|e| {
            Url::parse(u)
                .ok()
                .is_some_and(|u| u.path().to_lowercase().ends_with(&format!(".{e}")))
        })
}
fn protect(item: &Wallpaper) -> bool {
    if item.rating == Rating::Safe {
        return true;
    }
    let Some(tags) = &item.tags else {
        return false;
    };
    !tags.iter().any(|t| {
        [
            "loli",
            "lolicon",
            "shota",
            "shotacon",
            "child",
            "underage",
            "minor",
            "toddler",
            "baby",
            "cub",
            "bestiality",
            "zoophilia",
        ]
        .contains(&t.as_str())
    })
}
fn accepted(config: &Config, items: Vec<Wallpaper>) -> Vec<Wallpaper> {
    items
        .into_iter()
        .filter(|i| protect(i) && config.policy.accepts(i))
        .collect()
}
fn blank(provider: &str, id: String) -> Wallpaper {
    Wallpaper {
        id,
        provider: provider.into(),
        title: provider.into(),
        thumbnail: String::new(),
        preview_url: String::new(),
        image: String::new(),
        landing: String::new(),
        rating: Rating::Unknown,
        tags: None,
        width: 0,
        height: 0,
        author: String::new(),
        license: "Kaynak görselin haklarını kontrol et".into(),
        local: false,
        created_at: 0,
    }
}
pub fn parse_booru(provider: &str, v: &Value) -> Result<Wallpaper> {
    if v["is_deleted"].as_bool() == Some(true)
        || v["is_pending"].as_bool() == Some(true)
        || v["is_banned"].as_bool() == Some(true)
    {
        bail!("Kullanılamayan kayıt");
    }
    let id = v["id"].as_u64().context("Geçersiz kimlik")?;
    let mut w = blank(provider, format!("{}:{id}", provider.to_lowercase()));
    w.title = format!("{provider} · {id}");
    w.created_at = v["created_at"]
        .as_i64()
        .unwrap_or_else(|| crate::providers::parse_date(&txt(v, "created_at")));
    let dan = matches!(provider, "AIBooru" | "Danbooru");
    w.rating = match (txt(v, "rating").as_str(), dan) {
        ("g", true) | ("general", _) | ("s", false) | ("safe", _) => Rating::Safe,
        ("s", true) | ("q", _) | ("sensitive", _) | ("questionable", _) => Rating::Sketchy,
        ("e", _) | ("explicit", _) => Rating::Adult,
        _ => Rating::Unknown,
    };
    w.tags = Some(
        txt(v, if dan { "tag_string" } else { "tags" })
            .split_whitespace()
            .take(512)
            .map(str::to_owned)
            .collect(),
    );
    w.thumbnail = txt(
        v,
        if dan {
            "preview_file_url"
        } else {
            "preview_url"
        },
    );
    w.preview_url = txt(v, if dan { "large_file_url" } else { "sample_url" });
    w.image = txt(v, "file_url");
    w.width = dim(v, if dan { "image_width" } else { "width" });
    w.height = dim(v, if dan { "image_height" } else { "height" });
    if let Some(variants) = v["media_asset"]["variants"].as_array() {
        for (kind, target) in [
            ("360x360", &mut w.thumbnail),
            ("sample", &mut w.preview_url),
            ("original", &mut w.image),
        ] {
            if target.is_empty()
                && let Some(x) = variants.iter().find(|x| x["type"] == kind)
            {
                *target = txt(x, "url");
            }
        }
    }
    let host = match provider {
        "Yande.re" => "yande.re",
        "Konachan" => "konachan.com",
        "AIBooru" => "aibooru.online",
        _ => "danbooru.donmai.us",
    };
    w.landing = if dan {
        format!("https://{host}/posts/{id}")
    } else {
        format!("https://{host}/post/show/{id}")
    };
    w.author = txt(v, if dan { "tag_string_artist" } else { "author" });
    if !image_url(&w.image) || !image_url(&w.thumbnail) {
        bail!("Desteklenmeyen veya gizli görsel");
    }
    if !image_url(&w.preview_url) {
        w.preview_url = w.image.clone();
    }
    if !protect(&w) {
        bail!("Engellenen içerik etiketi");
    }
    Ok(w)
}
pub fn search(
    network: &Network,
    provider: &str,
    config: &Config,
    page: u32,
    credential: &str,
    cancel: &AtomicBool,
) -> Result<SearchBatch> {
    let mut out = Vec::new();
    let more;
    match provider {
        "Konachan" | "Danbooru" | "Gelbooru" => {
            bail!("Bu kaynak erişim sorunları nedeniyle kaldırıldı")
        }
        "Yande.re" | "AIBooru" => {
            if !config.policy.safe && !config.policy.sketchy && !config.policy.adult {
                return Ok(SearchBatch {
                    rejected: 0,
                    items: vec![],
                    has_more: false,
                });
            }
            let dan = matches!(provider, "AIBooru" | "Danbooru");
            let host = match provider {
                "Yande.re" => "https://yande.re/post.json",
                "Konachan" => "https://konachan.com/post.json",
                "AIBooru" => "https://aibooru.online/posts.json",
                "Danbooru" => "https://danbooru.donmai.us/posts.json",
                _ => "https://gelbooru.com/index.php",
            };
            let creds: Value = serde_json::from_str(credential).unwrap_or(Value::Null);
            if provider == "Gelbooru"
                && (txt(&creds, "key").is_empty() || txt(&creds, "user").is_empty())
            {
                bail!("Ücretsiz Gelbooru hesabından API anahtarı ve kullanıcı kimliği gerekli");
            }
            let mut tag_parts = config
                .query
                .split_whitespace()
                .take(2)
                .map(str::to_owned)
                .collect::<Vec<_>>();
            match config.gallery_order.as_str() {
                "newest" => tag_parts.push(if dan { "order:id_desc" } else { "order:id" }.into()),
                "random" => tag_parts.push("order:random".into()),
                "source" => tag_parts.push("order:score".into()),
                _ => {}
            }
            let classes = [
                config.policy.safe,
                config.policy.sketchy,
                config.policy.adult,
            ];
            if classes.iter().filter(|v| **v).count() == 1 {
                let rating = if config.policy.safe {
                    if dan { "g" } else { "safe" }
                } else if config.policy.sketchy {
                    if dan { "s" } else { "questionable" }
                } else {
                    if dan { "e" } else { "explicit" }
                };
                tag_parts.push(format!("rating:{rating}"));
            }
            // Anonymous booru endpoints typically limit tags to 2 on Danbooru-likes; keep priority order.
            if dan && tag_parts.len() > 2 {
                tag_parts.truncate(2);
            }
            let tags = tag_parts.join(" ");
            let mut url = Url::parse(host)?;
            {
                let mut q = url.query_pairs_mut();
                q.append_pair("limit", "24")
                    .append_pair("tags", tags.trim());
                if provider == "Gelbooru" {
                    q.append_pair("page", "dapi")
                        .append_pair("s", "post")
                        .append_pair("q", "index")
                        .append_pair("json", "1")
                        .append_pair("pid", &(page - 1).to_string())
                        .append_pair("api_key", &txt(&creds, "key"))
                        .append_pair("user_id", &txt(&creds, "user"));
                } else {
                    q.append_pair("page", &page.to_string());
                    if provider == "Danbooru" && !txt(&creds, "key").is_empty() {
                        q.append_pair("api_key", &txt(&creds, "key"))
                            .append_pair("login", &txt(&creds, "user"));
                    }
                }
            }
            let data = network.json(url.as_str(), cancel)?;
            let list = if provider == "Gelbooru" {
                data["post"].as_array()
            } else {
                data.as_array()
            }
            .context("Kaynak yanıtı tanınmadı")?;
            more = list.len() >= 24;
            for value in list.iter().take(24) {
                check(cancel)?;
                if provider == "Gelbooru" {
                    let mut v = value.clone();
                    if let Some(obj) = v.as_object_mut() {
                        for k in ["file_url", "preview_url", "sample_url"] {
                            if let Some(s) = obj.get(k).and_then(Value::as_str)
                                && s.starts_with("//")
                            {
                                let val = format!("https:{s}");
                                obj.insert(k.into(), Value::String(val));
                            }
                        }
                    }
                    if let Ok(mut w) = parse_booru("Gelbooru", &v) {
                        w.landing = format!(
                            "https://gelbooru.com/index.php?page=post&s=view&id={}",
                            value["id"]
                        );
                        out.push(w);
                    }
                } else if let Ok(w) = parse_booru(provider, value) {
                    out.push(w);
                }
            }
        }
        "Cleveland" => {
            if !config.policy.unknown {
                return Ok(SearchBatch {
                    rejected: 0,
                    items: vec![],
                    has_more: false,
                });
            }
            let mut url = Url::parse("https://openaccess-api.clevelandart.org/api/artworks/")?;
            url.query_pairs_mut()
                .append_pair("has_image", "1")
                .append_pair("cc0", "1")
                .append_pair(
                    "search",
                    if config.query.is_empty() {
                        "landscape"
                    } else {
                        &config.query
                    },
                )
                .append_pair("limit", "12")
                .append_pair("skip", &((page - 1) * 12).to_string());
            let data = network.json(url.as_str(), cancel)?;
            let rows = data["data"]
                .as_array()
                .context("Cleveland yanıtı tanınmadı")?;
            more = data["info"]["total"]
                .as_u64()
                .map_or(rows.len() == 12, |n| page as u64 * 12 < n);
            for v in rows.iter().take(12) {
                check(cancel)?;
                if v["share_license_status"] != "CC0" {
                    continue;
                }
                let image = &v["images"]["print"];
                let web = &v["images"]["web"];
                let mut w = blank(provider, format!("cma:{}", v["id"]));
                w.title = txt(v, "title");
                w.image = txt(image, "url");
                w.thumbnail = txt(web, "url");
                w.preview_url = w.image.clone();
                w.width = txt(image, "width").parse().unwrap_or(0);
                w.height = txt(image, "height").parse().unwrap_or(0);
                w.landing = txt(v, "url");
                w.license = "CC0 · Cleveland Museum of Art".into();
                w.author = v["creators"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .map(|v| txt(v, "description"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default();
                w.tags = Some(vec![txt(v, "type"), txt(v, "technique"), "art".into()]);
                if image_url(&w.image) && image_url(&w.thumbnail) {
                    out.push(w);
                }
            }
        }
        "The Met" => {
            if !config.policy.unknown {
                return Ok(SearchBatch {
                    rejected: 0,
                    items: vec![],
                    has_more: false,
                });
            }
            let mut url =
                Url::parse("https://collectionapi.metmuseum.org/public/collection/v1.1/search")?;
            url.query_pairs_mut()
                .append_pair("hasImages", "true")
                .append_pair("isPublicDomain", "true")
                .append_pair(
                    "q",
                    if config.query.is_empty() {
                        "landscape"
                    } else {
                        &config.query
                    },
                )
                .append_pair("limit", "8")
                .append_pair("offset", &((page - 1) * 8).to_string());
            let data = network.json(url.as_str(), cancel)?;
            let ids = data["objectIDs"]
                .as_array()
                .context("The Met yanıtı tanınmadı")?;
            more = data["total"].as_u64().is_some_and(|n| page as u64 * 8 < n);
            for id in ids.iter().take(8).filter_map(Value::as_u64) {
                check(cancel)?;
                let detail = format!(
                    "https://collectionapi.metmuseum.org/public/collection/v1/objects/{id}"
                );
                let v = match network.json(&detail, cancel) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                if v["isPublicDomain"] != true {
                    continue;
                }
                let mut w = blank(provider, format!("met:{id}"));
                w.title = txt(&v, "title");
                w.image = txt(&v, "primaryImage");
                w.thumbnail = txt(&v, "primaryImageSmall");
                w.preview_url = w.image.clone();
                w.author = txt(&v, "artistDisplayName");
                w.landing = txt(&v, "objectURL");
                w.license = "CC0 · The Metropolitan Museum of Art".into();
                w.created_at = crate::providers::parse_date(&txt(&v, "metadataDate"));
                w.tags = Some(
                    v["tags"]
                        .as_array()
                        .map(|tags| {
                            tags.iter()
                                .map(|v| txt(v, "term"))
                                .chain([txt(&v, "classification"), txt(&v, "medium")])
                                .collect()
                        })
                        .unwrap_or_default(),
                );
                if image_url(&w.image) && image_url(&w.thumbnail) {
                    out.push(w);
                }
            }
        }
        "V&A" => {
            if !config.policy.unknown {
                return Ok(SearchBatch {
                    rejected: 0,
                    items: vec![],
                    has_more: false,
                });
            }
            let mut url = Url::parse("https://api.vam.ac.uk/v2/objects/search")?;
            url.query_pairs_mut()
                .append_pair(
                    "q",
                    if config.query.is_empty() {
                        "landscape"
                    } else {
                        &config.query
                    },
                )
                .append_pair("images_exist", "1")
                .append_pair("page_size", "12")
                .append_pair("page", &page.to_string());
            let data = network.json(url.as_str(), cancel)?;
            let rows = data["records"].as_array().context("V&A yanıtı tanınmadı")?;
            more = data["info"]["pages"]
                .as_u64()
                .is_some_and(|n| (page as u64) < n);
            for v in rows.iter().take(12) {
                if v["_images"]["imageResolution"] != "high" {
                    continue;
                }
                let base = txt(&v["_images"], "_iiif_image_base_url");
                if valid_url(&base).is_err() {
                    continue;
                }
                let mut w = blank(provider, format!("vam:{}", txt(v, "systemNumber")));
                w.title = txt(v, "_primaryTitle");
                w.thumbnail = format!("{base}full/!400,400/0/default.jpg");
                w.image = format!("{base}full/full/0/default.jpg");
                w.preview_url = w.image.clone();
                w.author = txt(&v["_primaryMaker"], "name");
                w.landing = format!(
                    "https://collections.vam.ac.uk/item/{}/",
                    txt(v, "systemNumber")
                );
                w.license = "V&A · kullanım koşulları ve kaynak görselin hakları".into();
                w.tags = Some(vec![
                    txt(v, "objectType"),
                    txt(v, "_primaryPlace"),
                    "art".into(),
                ]);
                out.push(w);
            }
        }
        "NekosAPI" => {
            let mut ratings = Vec::new();
            if config.policy.safe {
                ratings.push("safe");
            }
            if config.policy.sketchy {
                ratings.extend(["suggestive", "borderline"]);
            }
            if config.policy.adult {
                ratings.push("explicit");
            }
            if ratings.is_empty() {
                return Ok(SearchBatch {
                    rejected: 0,
                    items: vec![],
                    has_more: false,
                });
            }
            let mut u = Url::parse("https://api.nekosapi.com/v4/images")?;
            {
                let mut q = u.query_pairs_mut();
                q.append_pair("limit", "24")
                    .append_pair("offset", &((page - 1) * 24).to_string())
                    .append_pair("rating", &ratings.join(","));
                if !config.query.trim().is_empty() {
                    q.append_pair("tags", &config.query);
                }
            }
            let data = network.json(u.as_str(), cancel)?;
            let list = data["items"]
                .as_array()
                .context("NekosAPI yanıtı tanınmadı")?;
            more = data["count"]
                .as_u64()
                .map_or(list.len() == 24, |n| page as u64 * 24 < n);
            for v in list.iter().take(24) {
                let mut w = blank(provider, format!("nekos:{}", v["id"]));
                w.image = txt(v, "url");
                let sample = txt(v, "sample_url");
                let thumb = txt(v, "thumbnail_url");
                w.thumbnail = if image_url(&thumb) {
                    thumb
                } else if image_url(&sample) {
                    sample.clone()
                } else {
                    w.image.clone()
                };
                w.preview_url = if image_url(&sample) {
                    sample
                } else {
                    w.image.clone()
                };
                w.rating = match txt(v, "rating").as_str() {
                    "safe" => Rating::Safe,
                    "suggestive" | "borderline" => Rating::Sketchy,
                    "explicit" => Rating::Adult,
                    _ => Rating::Unknown,
                };
                w.tags = strings(&v["tags"]);
                w.author = txt(v, "artist_name");
                w.landing = txt(v, "source_url");
                w.width = dim(v, "width");
                w.height = dim(v, "height");
                if image_url(&w.image) {
                    out.push(w);
                }
            }
        }
        "NASA" => {
            if !config.policy.safe {
                return Ok(SearchBatch {
                    rejected: 0,
                    items: vec![],
                    has_more: false,
                });
            }
            let mut u = Url::parse("https://images-api.nasa.gov/search")?;
            u.query_pairs_mut()
                .append_pair(
                    "q",
                    if config.query.is_empty() {
                        "nebula"
                    } else {
                        &config.query
                    },
                )
                .append_pair("media_type", "image")
                .append_pair("page_size", "24")
                .append_pair("page", &page.to_string());
            let data = network.json(u.as_str(), cancel)?;
            let list = data["collection"]["items"]
                .as_array()
                .context("NASA yanıtı tanınmadı")?;
            more = data["collection"]["links"]
                .as_array()
                .is_some_and(|v| v.iter().any(|v| v["rel"] == "next"));
            for v in list.iter().take(24) {
                let d = &v["data"][0];
                let id = txt(d, "nasa_id");
                let mut w = blank(provider, format!("nasa:{id}"));
                w.title = txt(d, "title");
                w.created_at = crate::providers::parse_date(&txt(d, "date_created"));
                w.rating = Rating::Safe;
                w.tags = strings(&d["keywords"]);
                w.author = txt(d, "secondary_creator");
                w.license =
                    "NASA kaynak kredisi; üçüncü taraf hakları ayrıca kontrol edilmeli".into();
                w.landing = format!("https://images.nasa.gov/details/{id}");
                if let Some(links) = v["links"].as_array() {
                    let orig = links
                        .iter()
                        .find(|x| x["rel"] == "canonical")
                        .or_else(|| links.first());
                    if let Some(o) = orig {
                        w.image = txt(o, "href");
                        w.width = dim(o, "width");
                        w.height = dim(o, "height");
                    }
                    w.thumbnail = links
                        .iter()
                        .find(|x| x["rel"] == "preview")
                        .map(|x| txt(x, "href"))
                        .unwrap_or_else(|| w.image.clone());
                    w.preview_url = links
                        .iter()
                        .find(|x| x["width"].as_u64() == Some(1280))
                        .map(|x| txt(x, "href"))
                        .unwrap_or_else(|| w.image.clone());
                }
                if image_url(&w.image) && image_url(&w.thumbnail) {
                    out.push(w);
                }
            }
        }
        "Bing" => {
            if page > 2 || !config.policy.safe {
                return Ok(SearchBatch {
                    rejected: 0,
                    items: vec![],
                    has_more: false,
                });
            }
            let u = format!(
                "https://www.bing.com/HPImageArchive.aspx?format=js&idx={}&n=4&mkt=en-US&uhd=1",
                (page - 1) * 4
            );
            let data = network.json(&u, cancel)?;
            let list = data["images"].as_array().context("Bing yanıtı tanınmadı")?;
            more = page < 2 && list.len() == 4;
            let needle = config.query.trim().to_lowercase();
            for v in list.iter().take(4) {
                if v["wp"].as_bool() != Some(true) {
                    continue;
                }
                let base = txt(v, "urlbase");
                if !base.starts_with("/th?id=OHR.") {
                    continue;
                }
                let mut w = blank(provider, format!("bing:{}", txt(v, "hsh")));
                w.title = txt(v, "title");
                w.thumbnail = format!("https://www.bing.com{base}_320x180.jpg");
                w.preview_url = format!("https://www.bing.com{base}_1280x720.jpg");
                w.image = format!("https://www.bing.com{base}_UHD.jpg");
                w.width = 3840;
                w.height = 2160;
                w.created_at = parse_yyyymmdd(&txt(v, "startdate"));
                w.landing = txt(v, "copyrightlink");
                w.author = txt(v, "copyright");
                w.license = "Bing / fotoğrafçının telif hakları".into();
                w.rating = Rating::Safe;
                w.tags = Some(vec![]);
                if needle.is_empty()
                    || w.title.to_lowercase().contains(&needle)
                    || w.author.to_lowercase().contains(&needle)
                {
                    out.push(w);
                }
            }
        }
        "Commons" => {
            if !config.policy.unknown {
                return Ok(SearchBatch {
                    rejected: 0,
                    items: vec![],
                    has_more: false,
                });
            }
            let mut u = Url::parse("https://commons.wikimedia.org/w/api.php")?;
            u.query_pairs_mut()
                .append_pair("action", "query")
                .append_pair("format", "json")
                .append_pair("generator", "search")
                .append_pair(
                    "gsrsearch",
                    &format!(
                        "{} filetype:bitmap",
                        if config.query.is_empty() {
                            "landscape"
                        } else {
                            &config.query
                        }
                    ),
                )
                .append_pair("gsrnamespace", "6")
                .append_pair("gsrlimit", "24")
                .append_pair("gsroffset", &((page - 1) * 24).to_string())
                .append_pair("prop", "imageinfo")
                .append_pair("iiprop", "url|size|timestamp|extmetadata")
                .append_pair("iiurlwidth", "640");
            let data = network.json(u.as_str(), cancel)?;
            more = data["continue"].is_object();
            if let Some(pages) = data["query"]["pages"].as_object() {
                for v in pages.values() {
                    let info = &v["imageinfo"][0];
                    let mut w = blank(provider, format!("commons:{}", v["pageid"]));
                    w.title = txt(v, "title").trim_start_matches("File:").into();
                    w.image = txt(info, "url");
                    w.thumbnail = txt(info, "thumburl");
                    w.preview_url = w.thumbnail.clone();
                    w.landing = txt(info, "descriptionurl");
                    w.width = dim(info, "width");
                    w.height = dim(info, "height");
                    w.created_at = crate::providers::parse_date(&txt(info, "timestamp"));
                    w.author = strip_html(&txt(&info["extmetadata"]["Artist"], "value"));
                    w.license = txt(&info["extmetadata"]["LicenseShortName"], "value");
                    w.tags = Some(
                        txt(&info["extmetadata"]["Categories"], "value")
                            .split('|')
                            .map(str::to_owned)
                            .collect(),
                    );
                    if valid_url(&w.image).is_ok() && valid_url(&w.thumbnail).is_ok() {
                        out.push(w);
                    }
                }
            }
        }
        _ => bail!("Kaynak tanınmadı"),
    }
    let count = out.len();
    let items = accepted(config, out);
    let rejected = count - items.len();
    Ok(SearchBatch {
        rejected,
        items,
        has_more: more,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn booru_ratings_are_distinct_and_underage_explicit_tags_are_rejected() {
        let mut v = serde_json::json!({"id":10,"rating":"s","tag_string":"adult mature_female","file_url":"https://example.org/a.png","preview_file_url":"https://example.org/t.jpg","image_width":4096,"image_height":2160});
        assert_eq!(parse_booru("Danbooru", &v).unwrap().rating, Rating::Sketchy);
        v["rating"] = serde_json::json!("e");
        let item = parse_booru("Danbooru", &v).unwrap();
        assert_eq!(item.rating, Rating::Adult);
        assert!(!Config::default().policy.accepts(&item));
        v["tag_string"] = serde_json::json!("loli");
        assert!(parse_booru("Danbooru", &v).is_err());
    }
    #[test]
    fn unsupported_or_hidden_records_do_not_poison_other_results() {
        let good = serde_json::json!({"id":1,"rating":"s","tags":"landscape","file_url":"https://example.org/a.jpg","preview_url":"https://example.org/t.jpg","sample_url":"https://example.org/s.jpg","width":3840,"height":2160});
        let mut bad = good.clone();
        bad["file_url"] = Value::Null;
        let out = [bad, good]
            .iter()
            .filter_map(|v| parse_booru("Yande.re", v).ok())
            .collect::<Vec<_>>();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].preview_url, "https://example.org/s.jpg");
    }
}
