use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{self, BufReader, Write};
use std::process;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal;
use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::tag::ItemKey;
use rodio::{Decoder, OutputStream, Sink};

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
    let duration = tagged_file.properties().duration();
    let length = format_time(duration);

    // print all tags
    show("artist: ", tag.artist().as_deref());
    show("album: ", tag.album().as_deref());
    show("album artist: ", tag.get_string(&ItemKey::AlbumArtist));
    show("genre: ", tag.genre().as_deref());
    show("title: ", tag.title().as_deref());
    show("track: ", Some(track.as_str()));
    show("length: ", Some(length.as_str()));

    // play the song
    println!();
    println!("space: pause   +/-: volume   q: quit");
    println!();

    let result = play(&path, duration);
    println!();

    if let Err(e) = result{
        eprintln!("could not play '{}': {}", path, e);
        process::exit(1);
    }
}

// print one line or "unknown" if value missing
fn show(label: &str, value: Option<&str>){
    println!("{:<14}{}", label, value.unwrap_or("unknown"));
}

// turns a duration into 'm::ss'
fn format_time(d:Duration)->String{
    let secs = d.as_secs();
    format!("{}:{:02}", secs / 60, secs % 60)
}

// plays audio file until it ends
fn play(path: &str, total: Duration) -> Result<(), Box<dyn Error>>{
    let(_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;
    let file = File::open(path)?;
    let source = Decoder::new(BufReader::new(file))?;

    sink.append(source);
    
    let _raw = RawMode::enable()?;

    loop{
        // react to a key, if one is pressed
        if event::poll(Duration::from_millis(100))?{
            if let Event::Key(key) = event::read()?{
                if key.kind == KeyEventKind::Press{
                    match key.code{
                        KeyCode::Char(' ') =>{
                            if sink.is_paused(){
                                sink.play();
                            }
                            else{
                                sink.pause();
                            }
                        }
                        KeyCode::Char('+') | KeyCode::Char('=') =>{
                            let volume = (sink.volume() + 0.05).min(1.0);
                            sink.set_volume(volume);
                        }
                        KeyCode::Chat('-') =>{
                            let volume = (sink.volume() - 0.05).max(0.0);
                            sink.set_volume(volume);
                        }
                        KeyCode::Char('q') => break,
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                        _ => {}
                    }
                }
            }
        }
        // stop when the song is over
        if sink.empty(){
            break;
        }
        // redraw the status line
        let state = if sink.is_paused(){"paused "} else{"playing"};
        let volume = (sink.volume() * 100.0).round() as u32;
        print!(
            "\r{}  {}/{}   volume{:>3}%  ",
            state,
            format_time(sink.get_pos()),
            format_time(total),
            volume
        );
        io::stdout().flush()?;
    }
    Ok(())
}

// turns raw mode on, and back off automatically when dropped
struct RawMode;

impl RawMode{
    fn enable() -> io::Result<RawMode>{
        terminal::enable_raw_mode()?;
        Ok(RawMode)
    }
}
impl Drop for RawMode{
    fn drop(&mut self){
        let _ = terminal::disable_raw_mode();
    }
}
