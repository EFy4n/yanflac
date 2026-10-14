use std::env;
use std::process;

use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::tag::ItemKey;

fn main(){
    // get path from command line
    let path = match env::args().nth(1){
        Some(p) => p,
        None => {
            eprintln!("usage: yanflac <audio-file>");
            process::exit(1);
        }
    };

    // open file and read it
    let tagged_file = match Probe::open(&path).and_then(|probe| probe.read()){
        Ok(file) => file,
        Err(e) => {
            eprintln!("could not read '{}': {}", path, e);
            process::exit(1);
        }
    };

    // pick tags (FLAC use Vorbis)
    let tag = match tagged_file.primary_tag().or_else(|| tagged_file.first_tag()){
        Some(t) => t,
        None => {
            eprintln!("no tags found in '{}'", path);
            process::exit(1);
        }
    };

    // build track number
    let track = match(tag.track(), tag.track_total()){
        (Some(n), Some(total)) => format!("{}/{}", n, total),
        (Some(n), None) => n.to_string(),
        _ => "unknown". to_string(),
    };

    // length of the song as m:s
    let secs = tagged_file.properties().duration().as_secs();
    let length = format!("{}:{:02}", secs / 60, secs % 60);

    // print all tags
    show("artist: ", tag.artist().as_deref());
    show("album: ", tag.album().as_deref());
    show("album artist: ", tag.get_string(&ItemKey::AlbumArtist));
    show("genre: ", tag.genre().as_deref());
    show("title: ", tag.title().as_deref());
    show("track: ", Some(track.as_str()));
    show("length: ", Some(length.as_str()));
}

// print one line or "unknown" if value missing
fn show(label: &str, value: Option<&str>){
    println!("{:<14}{}", label, value.unwrap_or("unknown"));
}








