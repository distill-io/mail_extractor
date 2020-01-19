//! # Mail Extractor
//!
//! `Mail_extractor` is a Rust library to extract files
//!  from MIME type files and returns you a hashmap
//!  which will contain filename and it corresponding file content as bytes.
//!
//! # Example
//!
//! ```
//! use std::collections::HashMap;
//! use mail_extractor;
//! fn get_files(file_stream: Vec<u8>) -> HashMap<String, Vec<u8>> {
//! 	let extracted_file: HashMap<String, Vec<u8>> = mail_extractor::rewrite(file_stream);    
//! 	extracted_file
//! }
//! ```

use lol_html::{element, HtmlRewriter, Settings};
use mailparse::body::Body;
use mailparse::parse_mail;
use regex::{Captures, Regex};
use htmlescape::decode_html;
use std::collections::HashMap;
use std::hash::Hash;
use uuid::Uuid;

#[macro_use]
extern crate lazy_static;
extern crate base64;

fn proxify_css<'a>(
	body: &str,
	link_to_hash_old: &mut HashMap<String, String>,
) -> (String, HashMap<String, String>) {
	let mut link_to_hash: HashMap<String, String> = HashMap::new();
	lazy_static! {
		static ref RE: Regex =
			Regex::new(r#"(?i)(?m)url\s*\(\s*([$&+,:;=?@#'"<>*%!/.-a-zA-Z_]+)\s*\)"#).unwrap();
	}
	(
		RE.replace_all(body, |caps: &Captures| {
			let match_url = &caps[0][5..caps[0].len() - 2];
			// let len = match_url.len();
			if match_url.starts_with("data:") | match_url.ends_with(".css") != true {
				if !link_to_hash_old.contains_key(match_url) {
					let hash: &str = &Uuid::new_v4().to_string();
					// let mut filename = match_url.split('/');
					let decoded_url = match decode_html(&match_url.to_string()) {
						Ok(new_name) => new_name,
						Err(_) => match_url.to_string(),
					};
					link_to_hash.insert(decoded_url, hash.to_string());
					format!("url(\"{}\")", hash)
				} else {
					format!("url(\"{}\")", link_to_hash_old.get(match_url).unwrap())
				}
			} else {
				format!("{}", &caps[0])
			}
		})
		.into_owned(),
		link_to_hash,
	)
}

fn rewrite_html(body: Vec<u8>) -> (Vec<u8>, HashMap<String, String>) {
	let mut html_out: Vec<u8> = vec![];
	let mut link_to_hash: HashMap<String, String> = HashMap::new();
	let mut rewriter = HtmlRewriter::try_new(
		Settings {
			element_content_handlers: vec![element!(
				"base, img[src], link[rel=stylesheet][href], iframe[src]",
				|el| {
					if el.get_attribute("rel") == Some("stylesheet".to_string()) {
						let src = el.get_attribute("href").unwrap();
						let length = src
							.split("/")
							.collect::<Vec<&str>>()
							.pop()
							.unwrap()
							.split('?')
							.next()
							.unwrap()
							.split('.')
							.collect::<Vec<&str>>()
							.len();
						let (src1, mut newname) = set_filename(src, ".css".to_string());
						let hash = Uuid::new_v4();
						if length == 1 {
							newname = hash.to_string() + &newname;
							link_to_hash.insert(el.get_attribute("href").unwrap(), newname.clone());
							el.set_attribute("href", &newname).unwrap();
						} else {
							link_to_hash.insert(el.get_attribute("href").unwrap(), newname);
							el.set_attribute(
								"href",
								&(src1.replace("#", "") + &String::from(".css")),
							)
							.unwrap();
						}
					} else if el.tag_name() == "iframe" {
						let src = el.get_attribute("src").unwrap();
						let (src1, newname) = set_filename(src, ".html".to_string());
						println!("{}", newname.clone());
						link_to_hash.insert(
							el.get_attribute("src")
								.unwrap()
								.replace("cid:", "<")
								.to_string() + ">",
							newname,
						);

						el.set_attribute("src", &(src1.replace("#", "") + &String::from(".html")))
							.unwrap();
					} else if el.tag_name() == "base" {
						el.set_attribute("href", "").unwrap();
					} else {
						let decoded_url = decode_html(&el.get_attribute("src").unwrap());
						let hash: &str = &Uuid::new_v4().to_string();
						// println!("{:?} {:?}", el.get_attribute("src").unwrap(), hash.to_string());
						link_to_hash.insert(decoded_url.unwrap(), hash.to_string());
						// link_to_hash.insert(el.get_attribute("src").unwrap(), hash.to_string());
						el.set_attribute("src", &hash.to_string()).unwrap();
					}
					Ok(())
				}
			)],
			..Settings::default()
		},
		|c: &[u8]| html_out.extend_from_slice(c),
	)
	.unwrap();

	rewriter.write(&body).unwrap();
	rewriter.end().unwrap();
	drop(rewriter);
	(html_out, link_to_hash)
}

fn merge<K: Hash + Eq + Clone, V: Clone>(
	first_context: &mut std::collections::HashMap<K, V>,
	second_context: &HashMap<K, V>,
) {
	for (key, value) in second_context.iter() {
		&first_context.insert(key.clone(), value.clone());
	}
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
	/*!
	 * Rewrites file and returns it's corresponding file content as bytes.
	*/

	let mut extracted_file: HashMap<String, Vec<u8>> = HashMap::new();
	let parsed = parse_mail(&mht_file).unwrap();
	let (index_out, mut link_to_hash) = rewrite_html(parsed.subparts[0].get_body_raw().unwrap());
	extracted_file.insert("index.html".to_string(), index_out);
	for sub in parsed.subparts {
		// println!("{:?} {:?}", sub.headers[2].get_value(), sub.headers[1].get_value());
		let name = match decode_html(&sub.headers[2].get_value().unwrap()) {
			Ok(new_name) => new_name,
			Err(_) => sub.headers[2].get_value().unwrap(),
		};
		let ctype = sub.headers[0].get_value().unwrap();
		let mut cid = match decode_html(&sub.headers[1].get_value().unwrap()) {
			Ok(new_name) => new_name,
			Err(_) => sub.headers[1].get_value().unwrap(),
		};
		// println!("{} {:?}",name,  cid);
		let creation;
		if link_to_hash.contains_key(&name) {
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
							let (after, link_to_hash_new) =
								proxify_css(css_content, &mut link_to_hash);
							merge(&mut link_to_hash, &link_to_hash_new);
							extracted_file.insert(src1.clone(), after.as_bytes().to_vec());
						} else if ctype == "text/html" {
							cid = cid[1..cid.len() - 1]
								.to_string()
								.split('.')
								.next()
								.unwrap()
								.to_string();
							let frame = cid + &String::from(".html");
							let frame_content = body.get_decoded_as_string().unwrap();
							let (after, link_to_hash_new) =
								rewrite_html(frame_content.as_bytes().to_vec());
							merge(&mut link_to_hash, &link_to_hash_new);
							extracted_file.insert(frame, after);
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
				let hash: &str = &Uuid::new_v4().to_string();
				let filename = match decode_html(&name) {
					Ok(new_name) => new_name,
					Err(_) => name,
				};
				link_to_hash.insert(filename, hash.to_string());
				match sub.get_body_encoded().unwrap() {
					Body::Base64(body) | Body::QuotedPrintable(body) => {
						extracted_file.insert(hash.to_string(), body.get_decoded().unwrap());
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
