use anyhow::Result;
use reqwest::blocking::get;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use webbrowser;

extern crate clap;
use clap::{
    app_from_crate, crate_authors, crate_description, crate_name, crate_version, Arg, SubCommand,
};

#[derive(Deserialize, Debug)]
struct Torrent {
    url: String,
    // overview:String,
    // provider:String,
    // source:String,
    // seeds:usize,
    // file:String,
}
#[derive(Deserialize, Debug)]
struct Episode {
    season: usize,
    episode: usize,
    title: String,
    torrents: HashMap<String, Torrent>,
}

#[derive(Deserialize, Debug)]
struct ShowResp {
    title: String,
    episodes: Vec<Episode>,
    year: String,
    last_updated: usize,
}

#[derive(Deserialize, Debug)]
struct MovieResp {
    title: String,
    torrents: HashMap<String, HashMap<String, Torrent>>,
    year: String,
}

#[derive(Deserialize, Debug)]
struct FailedResp {
    code: usize,
    // message:String,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum ShowRespRaw {
    Ok(ShowResp),
    Err(FailedResp),
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum MovieRespRaw {
    Ok(MovieResp),
    Err(FailedResp),
}

fn main() -> Result<()> {
    let m = app_from_crate!()
        .arg(
            Arg::with_name("domain")
                .short("d")
                .long("domain")
                .takes_value(true),
        )
        .subcommand(
            SubCommand::with_name("show")
                .arg(Arg::with_name("imdb_id").takes_value(true).required(true))
                .arg(
                    Arg::with_name("season")
                        .short("s")
                        .long("season")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("episode")
                        .short("e")
                        .long("episode")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("resolution")
                        .short("r")
                        .long("resolution")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("lang")
                        .short("l")
                        .long("lang")
                        .takes_value(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("movie")
                .arg(Arg::with_name("imdb_id").takes_value(true).required(true))
                .arg(
                    Arg::with_name("resolution")
                        .short("r")
                        .long("resolution")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("lang")
                        .short("l")
                        .long("lang")
                        .takes_value(true),
                ),
        )
        .get_matches();

    let mut domain = String::from("https://popcorn-ru.tk");
    if let Some(res) = m.value_of("domain") {
        domain = res.to_string();
    }
    let mut locale = String::from("en");
    let mut resolution = String::from("?");

    if let Some(matches) = m.subcommand_matches("show") {
        if let Some(res) = matches.value_of("resolution") {
            resolution = res.to_string();
        }
        if let Some(lang) = matches.value_of("lang") {
            locale = lang.to_string();
        }
        let imdb_id = matches.value_of("imdb_id").unwrap();
        let url = format!("{}/{}/{}?locale={}", &domain, "show", &imdb_id, &locale);
        let raw_resp: ShowRespRaw = get(url)?.json()?;
        let mut resp: ShowResp = match raw_resp {
            ShowRespRaw::Ok(val) => val,
            ShowRespRaw::Err(_msg) => {
                println!("{} not found", &imdb_id);
                return Ok(());
            }
        };
        let mut info: BTreeMap<usize, usize> = BTreeMap::new();
        for episode in resp.episodes.iter() {
            let item = info.entry(episode.season).or_insert(episode.season);
            if *item < episode.episode {
                *item = episode.episode;
            }
        }
        println!("\n {}\n", resp.title);
        let max_season = info.iter().last().unwrap_or((&0, &0)).0;
        if max_season > &0 {
            let s = if max_season == &1usize {
                "season"
            } else {
                "seasons"
            };
            println!(" {:?} {}:", &max_season, s);
            for (season, episodes) in info.range(1..) {
                println!("  {:?}. {:?}", season, episodes);
            }
        }
        println!("");

        if matches.is_present("season") && matches.is_present("episode") {
            let episode = matches.value_of("episode").unwrap().parse::<usize>()?;
            let season = matches.value_of("season").unwrap().parse::<usize>()?;
            resp.episodes
                .retain(|x| x.episode == episode && x.season == season);
            if resp.episodes.is_empty() {
                println!("Episode not found");
                return Ok(());
            }

            if let Some(torrent) = resp.episodes[0].torrents.get(&resolution) {
                println!("Opening magnet link in default browser...");
                webbrowser::open(&torrent.url)?;
                return Ok(());
            } else {
                let hint = if resolution != "?" {
                    "Selected resolution not found.\n"
                } else {
                    ""
                };
                println!(
                    "{}Available resolutions: {:?}",
                    hint,
                    &resp.episodes[0]
                        .torrents
                        .keys()
                        .filter(|x| x.contains("p"))
                        .collect::<Vec<&String>>()
                );
                return Ok(());
            }
        }
    }

    if let Some(matches) = m.subcommand_matches("movie") {
        if let Some(res) = matches.value_of("resolution") {
            resolution = res.to_string();
        }
        if let Some(lang) = matches.value_of("lang") {
            locale = lang.to_string();
        }
        let imdb_id = matches.value_of("imdb_id").unwrap();
        let url = format!("{}/{}/{}?locale={}", &domain, "movie", &imdb_id, &locale);
        let raw_resp: MovieRespRaw = get(url)?.json()?;
        let resp: MovieResp = match raw_resp {
            MovieRespRaw::Ok(val) => val,
            MovieRespRaw::Err(_msg) => {
                println!("{} not found", &imdb_id);
                return Ok(());
            }
        };
        println!("\n{}\n", resp.title);
        if resp.torrents.is_empty() {
            println!("{} locale not found", &locale);
            return Ok(());
        }
        if let Some(torrents) = resp.torrents.get(&locale) {
            if let Some(torrent) = torrents.get(&resolution) {
                println!("Opening magnet link in default browser...");
                webbrowser::open(&torrent.url)?;
                return Ok(());
            } else {
                let hint = if resolution != "?" {
                    "Selected resolution not found.\n"
                } else {
                    ""
                };
                println!(
                    "{}Available resolutions: {:?}",
                    hint,
                    &torrents
                        .keys()
                        .filter(|x| x.contains("p"))
                        .collect::<Vec<&String>>()
                );
                return Ok(());
            }
        }
    }
    Ok(())
}
