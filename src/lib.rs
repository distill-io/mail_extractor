use lol_html::{element, HtmlRewriter, Settings};
use mailparse::body::Body;
use mailparse::parse_mail;
use regex::{Captures, Regex};
use std::borrow::Cow;

use std::collections::HashMap;

#[macro_use]
extern crate lazy_static;
extern crate base64;

fn proxify_css(body: &str) -> Cow<str> {
	lazy_static! {
		static ref RE: Regex =
			Regex::new(r#"(?i)(?m)url\s*\(\s*([$&+,:;=?@#'"<>*%!/.-a-zA-Z_]+)\s*\)"#).unwrap();
	}
	RE.replace_all(body, |caps: &Captures| {
		let match_url = &caps[0][5..caps[0].len() - 2];
		// let len = match_url.len();
		if match_url.starts_with("data:") != true {
			let mut filename = match_url.split('/');
			let name = filename.next_back();
			format!("url(\"{}\")", name.unwrap())
		} else {
			format!("{}", &caps[0])
		}
	})
}

fn rewrite_html(body: Vec<u8>, link_to_hash: &mut HashMap<String, String>) -> (Vec<u8>, &mut HashMap<String, String>) {
	let mut html_out: Vec<u8> = vec![];
	let mut rewriter = HtmlRewriter::try_new(
		Settings {
			element_content_handlers: vec![element!(
				"base, img[src], link[rel=stylesheet][href], iframe[src]",
				|el| {
					if el.get_attribute("rel") == Some("stylesheet".to_string()) {
						let src = el.get_attribute("href").unwrap();
						let (src1, newname) = set_filename(src, ".css".to_string());

						link_to_hash.insert(el.get_attribute("href").unwrap(), newname);

						el.set_attribute(
							"href",
							&(src1.replace("#", "") + &String::from(".css")),
						)
						.unwrap();
					} else if el.tag_name() == "iframe" {
						let src = el.get_attribute("src").unwrap();
						let (src1, newname) = set_filename(src, ".html".to_string());

						link_to_hash.insert(
							el.get_attribute("src")
								.unwrap()
								.replace("cid:", "<")
								.to_string() + ">",
							newname,
						);

						el.set_attribute(
							"src",
							&(src1.replace("#", "") + &String::from(".html")),
						)
						.unwrap();
					} else if el.tag_name() == "base" {
						el.set_attribute("href", "").unwrap();
					} else {
						let src = el.get_attribute("src").unwrap();

						let mut filename = src.split('/');
						let name = filename.next_back();
						let src1 = name.unwrap();
						let filename1 = src1.split('?').next().unwrap();
						link_to_hash
							.insert(el.get_attribute("src").unwrap(), filename1.to_string());
						el.set_attribute("src", &filename1.to_string()).unwrap();
					}
					Ok(())
				}
			)],
			..Settings::default()
		},
		|c: &[u8]| html_out.extend_from_slice(c),
	)
	.unwrap();

	rewriter
		.write(&body)
		.unwrap();
	rewriter.end().unwrap();
	drop(rewriter);
	(html_out, link_to_hash)
}

fn parse_by_slash(path: String) -> String {
	let mut filename = path.split('/');
	let name = filename.next_back();
	name.unwrap().to_string()
}

fn set_filename(filename: String, file_format: String) -> (String, String) {
	let mut filename2 = filename.split('/');
	let name = filename2.next_back();
	let src1 = name.unwrap();
	let filename1 = src1.split('?').next().unwrap();

	let mut filename = filename1.split(':');
	let name = filename.next_back().unwrap();
	let mut name1 = name.split('.');
	let src1 = name1.next();
	(
		src1.unwrap().to_string(),
		String::from(src1.unwrap().to_string().replace("#", "")) + &String::from(file_format),
	)
}

pub fn rewrite(mht_file: Vec<u8>) -> HashMap<String, Vec<u8>> {
	let mut extracted_file: HashMap<String, Vec<u8>> = HashMap::new();

	let parsed = parse_mail(&mht_file).unwrap();

	let mut link_to_hash: HashMap<String, String> = HashMap::new();
	
	let (index_out, link_to_hash) = rewrite_html(parsed.subparts[0].get_body_raw().unwrap(), &mut link_to_hash);
	extracted_file.insert("index.html".to_string(), index_out);
	for sub in parsed.subparts {
		let name = sub.headers[2].get_value().unwrap();
		let ctype = sub.headers[0].get_value().unwrap();
		let cid = sub.headers[1].get_value().unwrap();

		let creation;
		if link_to_hash.get(&name) != None {
			creation = link_to_hash.get(&name);
		} else {
			creation = link_to_hash.get(&cid);
		}
		match creation {
			Some(name1) => {
				let mut filename = name1.split(':');
				let name = filename.next_back();
				let src1 = name.unwrap().replace("#", "");
				match sub.get_body_encoded().unwrap() {
					Body::Base64(body) | Body::QuotedPrintable(body) => {
						if ctype == "text/css" {
							let css_content: &str = &*body.get_decoded_as_string().unwrap();
							let after = proxify_css(css_content);
							extracted_file.insert(src1.clone(), after.as_bytes().to_vec());
						} else if ctype == "text/html" {
							let frame_content = body.get_decoded_as_string().unwrap();
							let (after, link_to_hash) = rewrite_html(frame_content.as_bytes().to_vec(), link_to_hash);
							extracted_file.insert(src1.clone(), after);
							// println!("{:?}", std::str::from_utf8(&after));
						} else {
							extracted_file.insert(src1.clone(), body.get_decoded().unwrap());
						}
					}
					Body::SevenBit(body) | Body::EightBit(body) => {
						println!("mail body: {:?}", body.get_raw());
					}
					Body::Binary(body) => {
						println!("mail body binary: {:?}", body.get_raw());
					}
				}
			}
			None => {
				let filename = parse_by_slash(name);
				match sub.get_body_encoded().unwrap() {
					Body::Base64(body) | Body::QuotedPrintable(body) => {
						extracted_file.insert(filename.clone(), body.get_decoded().unwrap());
					}
					Body::SevenBit(body) | Body::EightBit(body) => {
						println!("mail body: {:?}", body.get_raw());
					}
					Body::Binary(body) => {
						println!("mail body binary: {:?}", body.get_raw());
					}
				}
			}
		}
	}
	extracted_file
}
