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
    println!("space: pause  +/-: volume  #sh #sl #mh #ml: seek  esc: cancel  q: quit");
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
    let mut pending = Pending::Empty;

    loop{
        // react to a key, if one is pressed
        if event::poll(Duration::from_millis(100))?{
            if let Event::Key(key) = event::read()?{
                if key.kind == KeyEventKind::Press{
                    match key.code{
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
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
                        KeyCode::Esc => pending = Pending::Empty,
                        KeyCode::Char('c') => {
                            if let Some(action) = handle_char(&mut pending, c){
                                match action{
                                    Action::SeekBy(seconds) => seek_by(&sink, seconds, total),
                                }                            
                            }
                        }
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

// command parser
// the unit of seek command
#[derive(Clone, Copy)]
enum Unit{
    Seconds,
    Minutes,
}

impl Unit{
    // how many seconds one of this unit is
    fn seconds(self) -> i64{
        match self{
            Unit::Seconds => 1,
            Unit::Minutes => 60,
        }
    }
    fn letter(self) -> &'static str{
        match self{
            Unit::Seconds => "s",
            Unit::Minutes => "m",
        }
    }
}

// what has been typed so far
#[derive(Clone, Copy)]
enum Pending{
    Empty,                       // nothing typed
    Count(64),                   // a number like '12'
    WithUnit(Option<u64>, Unit), // maybe a number, then s or m, example: '12s' or '12m'
}

impl Pending{
    // the text shown in the status line (corner), like in vim
    fn display(self) -> String{
        match self{
            Pending::Empty => String::new(),
            Pending::Count(n) => n.to_string(),
            Pending::WithUnit(Some(n), unit) => format!("{}{}", n, unit.letter()),
            Pending::WithUnit(None, unit) => unit.letter().to_string(),
        }
    }
}

// a finished command that is ready to run
enum Action{
    SeekBy(i64), // seconds (negative means back)
}

/* 
*  feeds one typed character into the parser
*  returns an 'Action' when a command is complete
*/
fn handle_char(pending: &num Pending, c: char) -> Option<Action>{
    let(next, action) = match(*pending, c)
        // digits build up the count
        (Pending::Empty, '0'..='9') => {
            let digit = c.to_digit(10).unwrap() as u64;
            (Pending::Count(digit), None)
        }
        (Pending::Count(n), '0'..='9') => {
            let digit = c.to_digit(10).unwrap() as u64;
            (Pending::Count((n * 10 + digit).min(99_999)), None)
        }

        // s or m picks the unit
        (Pending::Empty, 's') => (Pending::WithUnit(None, Unit::Seconds), None),
        (Pending::Empty, 'm') => (Pending::WithUnit(None, Unit::Minutes), None),

        (Pending::Count(n), 's') => (Pending::WithUnit(Some(n), Unit::Seconds), None),
        (Pending::Count(n), 'm') => (Pending::WithUnit(Some(n), Unit::Minutes), None),

        // h or l after a unit finishes the command
        (Pending::WithUnit(count, unit), 'h' | 'l') => {
            let seconds = count.unwrap_or(1) as i64 * unit.seconds();
            let offset = if == 'h' (-seconds) else {seconds};
            (Pending::Empty, Some(Action::Seekby(offset)))
        }

        // anything else cancels
        _ => (Pending::Empty, None).
    };
    *pending = next;
    action
}

// jumps forward or back, staying inside the song
fn seek_by(sink: &sink, offset: i64, total: Duration){
    let current = sink.get_pos().as_secs() as i64;
    let last = total.as_secs().saturating_sub(1) as i64;
    let target = (current + offset).clamp(0, last);

    // failed seek not worth stop the music, ignore it
    let _ = sink.try_seek(Duration::from_secs(target as u64));
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
