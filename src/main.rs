extern crate html5ever;
extern crate hyper;
extern crate hyper_native_tls;
extern crate regex;
extern crate string_cache;

use html5ever::rcdom::{Handle, NodeData};
use html5ever::tendril::TendrilSink;
use hyper::net::HttpsConnector;
use hyper::Client;
use hyper_native_tls::NativeTlsClient;
use regex::Regex;
use std::env;

fn main() {
    let query = match parse_args() {
        Some(arg) => arg,
        None => {
            println!("Enter your query.");
            return ();
        }
    };
    let doc = match get_database_response(&query) {
        Some(document) => document,
        None => {
            println!("Something awful happened with server, so we cant satisfy your query :(");
            return ();
        }
    };
    let result = get_info(doc);
    let result = beautify(&result);
    println!("{}", result);
}

fn parse_args() -> Option<String> {
    let args: Vec<String> = env::args().collect();
    if args.len() == 1 {
        None
    } else {
        Some(args[1].clone())
    }
}

fn get_database_response(query: &str) -> Option<Handle> {
    let ssl = NativeTlsClient::new().unwrap();
    let connector = HttpsConnector::new(ssl);
    let client = Client::with_connector(connector);

    let mut database = String::from("https://en.wikipedia.org/wiki/");
    database.push_str(query);

    let mut resp = match client.get(&database).send() {
        Ok(response) => response,
        Err(_) => return None,
    };
    match html5ever::parse_document(html5ever::rcdom::RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut resp)
    {
        Ok(dom) => Some(dom.document),
        Err(_) => None,
    }
}

fn get_info(handle: Handle) -> String {
    let mut worth_starting = true;
    let mut result = String::new();
    let mut next_ready = false;
    parse(
        false,
        &mut worth_starting,
        &mut next_ready,
        &handle,
        &mut result,
    );
    result
}

fn parse(
    is_important: bool,
    is_searching_info: &mut bool,
    is_next_important: &mut bool,
    handle: &Handle,
    collector: &mut String,
) {
    if !is_important && !*is_searching_info {
        return ();
    }
    let node = handle;
    if let NodeData::Text { ref contents } = node.data {
        if is_important {
            collector.push_str(&contents.borrow());
        }
    }

    let mut next_command = is_important;
    if let NodeData::Element {
        ref name,
        ref attrs,
        ..
    } = node.data
    {
        if name.local.eq_str_ignore_ascii_case("p") && attrs.borrow().is_empty() {
            if *is_searching_info && *is_next_important {
                next_command = true;
                *is_searching_info = false;
                *is_next_important = false;
            }
        } else if name.local.eq_str_ignore_ascii_case("table") {
            *is_next_important = true;
        }
    }

    for child in node.children.borrow().iter() {
        parse(
            next_command,
            is_searching_info,
            is_next_important,
            child,
            collector,
        );
    }
}

fn beautify(target: &String) -> String {
    let mut consumer = String::new();
    {
        let mut in_brace = 0;
        for chr in target.chars() {
            match chr {
                '(' => in_brace += 1,
                ')' => in_brace -= 1,
                '[' => in_brace += 1,
                ']' => in_brace -= 1,
                _ => if in_brace == 0 {
                    consumer.push(chr)
                },
            }
        }
    }
    let target = consumer;
    let escaper = Regex::new(r"\\n").unwrap();
    let spacer = Regex::new(r"  ").unwrap();
    let comma_splitter = Regex::new(r" ,").unwrap();
    let target = escaper.replace_all(&target, "");
    let target = spacer.replace_all(&target, " ");
    let target = comma_splitter.replace_all(&target, ",");
    if target.ends_with(":") {
        return String::from(
            "Your message was not recognised by database. \nPlease, correct it or enter another one.",
        );
    }
    let target = target.replace("\\\"", "\"");
    target.replace("\\'", "'")
}
