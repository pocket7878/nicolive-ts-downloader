extern crate reqwest;
extern crate sxd_document;
extern crate sxd_xpath;
extern crate cookie;

use std::io::Read;
use reqwest::header::{SetCookie, Cookie};
use sxd_document::parser;
use sxd_xpath::{evaluate_xpath, Value};
use std::collections::HashMap;

fn gets(prompt: &str) -> String {
    println!("{}",prompt);
    let mut input_buf = String::new();
    std::io::stdin()
        .read_line(&mut input_buf)
        .expect("failed to read from stdin");
    return input_buf
}

/*
fn get_nico_live_server_time() -> String {
    let mut server_time_result = reqwest::get("http://live.nicovideo.jp/api/getservertime").unwrap();
    if !(server_time_result.status().is_success()) {
        panic!("Failed to get server time from nico live.");
    }

    let mut server_time_content = String::new();
    server_time_result.read_to_string(&mut server_time_content).unwrap();

    return server_time_content.trim_left_matches("servertime=").to_owned();
}

fn get_ticket(email: &str, pass: &str, server_time: &str) -> String {
    let params = [("mail", email), ("password", pass), ("site", "nicolive_encoder"), ("time", server_time)];
    let client = reqwest::Client::new().unwrap();
    let mut login_result = client.post("https://account.nicovideo.jp/api/v1/login").unwrap()
        .header(UserAgent::new("nicoliveenc/2.0.7"))
        .form(&params).unwrap()
        .send().unwrap();
    let mut login_content = String::new();
    login_result.read_to_string(&mut login_content).unwrap();

    // Parse login_content
    let login_xml = parser::parse(&login_content).expect("failed to parse XML");
    let login_document = login_xml.as_document();

    let ticket_value = evaluate_xpath(&login_document, "/nicovideo_user_response/ticket").expect("Retrieve ticket from login_xml failed.");
    return ticket_value.string();
}
*/

fn get_lv_status(email: &str, pass: &str, lv_num: &str) -> String {
    use std::time::Duration;
    use reqwest::RedirectPolicy;

    let params = [("mail", email), ("password", pass)];
    let client: reqwest::Client = reqwest::Client::builder().unwrap()
        .gzip(true)
        .redirect(RedirectPolicy::none())
        .timeout(Duration::from_secs(10))
        .build().unwrap();
    let login_result = client.post("https://account.nicovideo.jp/api/v1/login").unwrap()
        .form(&params).unwrap()
        .send().unwrap();

    let set_cookies = login_result.headers().iter()
        .filter_map(|header| {
            if header.is::<SetCookie>() {
                header.value::<SetCookie>()
            } else {
                None
            }
        })
        .next();
    let mut new_cookie: Cookie = Cookie::new();
    match set_cookies {
        Some(v) => {
            for cookie in v.iter() {
                let c = cookie::Cookie::parse(cookie.clone()).unwrap();
                new_cookie.set(c.name().to_owned(), c.value().to_owned());
            }
        },
        None => {
        }
    }

    let mut lv_status_result = client.get(&format!("http://live.nicovideo.jp/api/getplayerstatus/lv{}", lv_num)).unwrap()
        .header(new_cookie)
        .send().unwrap();

    let mut lv_status_content = String::new();
    lv_status_result.read_to_string(&mut lv_status_content).unwrap();

    return lv_status_content
}

fn get_queues(doc: &sxd_document::dom::Document) -> HashMap<String, Vec<String>> {
    use Value::*;
    let queues = evaluate_xpath(&doc, "/getplayerstatus/stream/quesheet/que").expect("Retrieve queue from lv_status failed.");
    let publish_list = match queues {
        String(ref val) => {
            vec![val.clone()]
        },
        Nodeset(ref ns) => {
            ns.document_order().iter().map(|n| n.string_value().clone()).filter(|s| s.starts_with("/publish")).collect::<Vec<_>>()
        },
        _ => {
            vec![]
        }
    };

    let mut queue_data: HashMap<std::string::String, Vec<std::string::String>> = HashMap::new();
    for publish in publish_list.iter() {
       let publish_data = publish.split(' ').collect::<Vec<_>>();
       let key = publish_data[1].to_owned();
       let value = publish_data[2].to_owned();
       if queue_data.contains_key(&key) {
           let mut old_vectors = queue_data.get_mut(&key).unwrap();
           old_vectors.push(value);
       } else {
           queue_data.insert(key, vec![value]);
       }
    }
    return queue_data
}

fn get_play_list(doc: &sxd_document::dom::Document) -> Vec<(String, String)> {
    use Value::*;
    let queues = evaluate_xpath(&doc, "/getplayerstatus/stream/quesheet/que").expect("Retrieve queue from lv_status failed.");
    match queues {
        Nodeset(ref ns) => {
            match ns.document_order().iter().map(|n| n.string_value().clone()).find(|s| s.starts_with("/play")) {
                Some(play_line) => {
                    let play_table_line = play_line.split(' ').collect::<Vec<_>>()[1];
                    let play_entries: Vec<&str> = play_table_line.trim_left_matches("case:").split(',').collect::<Vec<_>>();
                    let mut play_list = vec![];
                    for play_entry in play_entries.iter() {
                        let play = play_entry.split(':').collect::<Vec<_>>();
                        play_list.push((play[0].to_owned(), play[2].to_owned()));
                    }
                    return play_list
                },
                None => {
                    return vec![]
                }
            }
        },
        _ => {
            return vec![]
        }
    }
}

fn get_rtmp_urls(doc: &sxd_document::dom::Document) -> Vec<String> {
    use Value::*;

    let rtmp_urls = evaluate_xpath(&doc, "/getplayerstatus/rtmp/url").expect("Retrieve rtmp urls from lv_status failed.");
    match rtmp_urls {
        String(ref val) => {
            return vec![val.clone()]
        },
        Nodeset(ref ns) => {
            ns.document_order().iter().map(|n| n.string_value().clone()).collect::<Vec<_>>()
        },
        _ => {
            return vec![]
        }
    }
}

fn get_rtmp_tickets(doc: &sxd_document::dom::Document) -> Vec<String> {
    use Value::*;

    let rtmp_tickets = evaluate_xpath(&doc, "/getplayerstatus/rtmp/ticket").expect("Retrieve rtmp tickets from lv_status failed.");
    match rtmp_tickets {
        String(ref val) => {
            return vec![val.clone()]
        },
        Nodeset(ref ns) => {
            ns.document_order().iter().map(|n| n.string_value().clone()).collect::<Vec<_>>()
        },
        _ => {
            return vec![]
        }
    }
}

fn read_number(min: u32, max: u32) -> u32 {
    loop {
    let mut input_text = String::new();
    std::io::stdin()
        .read_line(&mut input_text)
        .expect("failed to read from stdin");

    let trimmed = input_text.trim();
    match trimmed.parse::<u32>() {
        Ok(i) if (i >= min && i <= max) => {
            return i
        }
        _ => { continue }
    };
    }
}

fn main() {
    use std::process::Command;
    let email = gets("Input email");
    let pass = gets("Input password");
    let lv_num = gets("Input lv number");

    let lv_status = get_lv_status(&email.trim(), &pass.trim(), &lv_num.trim());
    let lv_status_xml = parser::parse(&lv_status).expect("failed to parse XML");
    let lv_status_doc = lv_status_xml.as_document();

    let play_list = get_play_list(&lv_status_doc);
    let queues = get_queues(&lv_status_doc);
    let rtmp_urls = get_rtmp_urls(&lv_status_doc);
    let rtmp_tickets = get_rtmp_tickets(&lv_status_doc);

    //Ask user to choose queue
    let mut index = 1;
    for play_entry in play_list.iter() {
        println!("{}: {}", index, play_entry.0);
        index += 1;
    }
    let play_index = read_number(1, play_list.len() as u32);
    let file_name = gets("Input file name");
    let selected_play = &play_list[(play_index - 1) as usize].1;
    match queues.get(selected_play) {
        Some(qs) => {
            for (part, q) in qs.iter().enumerate() {
                let mut child = Command::new("rtmpdump")
                                    .arg("-r")
                                    .arg(rtmp_urls[0].clone())
                                    .arg("-y")
                                    .arg(&format!("mp4:{}", q))
                                    .arg("-C")
                                    .arg(&format!("S:{}", rtmp_tickets[0].clone()))
                                    .arg("-e")
                                    .arg("-o")
                                    .arg(&format!("{}_part{}.flv", file_name.trim().clone(), part + 1))
                                    .spawn()
                                    .expect("failed to execute process");
                let status = child.wait().unwrap();
                println!("{}", status);
            }
        },
        None => {
        }
    }
}
