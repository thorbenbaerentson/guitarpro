use fraction::ToPrimitive;

use crate::io::*;
use crate::key_signature::*;
use crate::effects::*;
use crate::lyric::*;
use crate::midi::*;
use crate::rse::*;

use std::collections::HashMap;

#[derive(Clone)]
pub struct Version {
    pub data: String,
    pub number: u8,
    pub clipboard: bool
}

pub const _VERSION_1_0X: u8 = 10;
pub const _VERSION_2_2X: u8 = 22;
pub const VERSION_3_00: u8 = 30;
pub const VERSION_4_0X: u8 = 40;
pub const VERSION_5_00: u8 = 50;
pub const VERSION_5_10: u8 = 51;

// Struct utility to read file: https://stackoverflow.com/questions/55555538/what-is-the-correct-way-to-read-a-binary-file-in-chunks-of-a-fixed-size-and-stor
#[derive(Clone)]
pub struct Song {
    pub version: Version,
    pub clipboard: Option<Clipboard>,

    pub name: String,
    pub subtitle: String, //Guitar Pro
	pub artist: String,
	pub album: String,
    pub words: String, //GP
	pub author: String, //music by
	pub date: String,
	pub copyright: String,
    /// Tab writer
	pub writer: String,
	pub transcriber: String,
    pub instructions: String,
	pub comments: String,
    pub notice: Vec<String>,

	pub tracks: Vec<Track>,
	pub measure_headers: Vec<MeasureHeader>,
	pub channels: Vec<MidiChannel>,
    pub lyrics: Lyrics,
    pub tempo: i16,
    pub hide_tempo: bool,
    pub tempo_name:String,
    pub key: KeySignature,

    pub triplet_feel: TripletFeel,
    pub current_measure_number: Option<u16>,
    pub current_track: Option<Track>,
    pub master_effect: RseMasterEffect,
}

impl Default for Song {
	fn default() -> Self { Song {
        version: Version {data: String::with_capacity(30), clipboard: false, number: 0}, clipboard: None,
		name:String::new(), subtitle: String::new(), artist:String::new(), album: String::new(),
        words: String::new(), author:String::new(), date:String::new(),
        copyright:String::new(), writer:String::new(), transcriber:String::new(), comments:String::new(),
        notice:Vec::new(),
        instructions: String::new(),
		tracks:Vec::new(),
		measure_headers:Vec::new(),
		channels:Vec::with_capacity(64),
        lyrics: Lyrics::default(),
        tempo: 120, hide_tempo: false, tempo_name:String::from("Moderate"),
        key: KeySignature::default(),

        triplet_feel: TripletFeel::NONE,
        current_measure_number: None, current_track: None,

        master_effect: RseMasterEffect::default(),
	}}
}

impl Song {
    /// Read and process version
    fn read_version(&mut self, data: &Vec<u8>, seek: &mut usize) {
        self.version = read_version(data, seek);
        let mut clipboard = Clipboard::default();
        //check for clipboard and read it
        if self.version.number == VERSION_4_0X && self.version.clipboard {
            clipboard.start_measure = read_int(data, seek);
            clipboard.stop_measure  = read_int(data, seek);
            clipboard.start_track = read_int(data, seek);
            clipboard.stop_track  = read_int(data, seek);
        }
        if self.version.number == VERSION_5_00 && self.version.clipboard {
            clipboard.start_beat = read_int(data, seek);
            clipboard.stop_beat  = read_int(data, seek);
            clipboard.sub_bar_copy = read_int(data, seek) != 0;
        }
    }
    /// Read meta information (name, artist, ...)
    fn read_meta(&mut self, data: &Vec<u8>, seek: &mut usize) {
        // read GP3 informations
        self.name        = read_int_size_string(data, seek);//.replace("\r", " ").replace("\n", " ").trim().to_owned();
        self.subtitle    = read_int_size_string(data, seek);
        self.artist      = read_int_size_string(data, seek);
        self.album       = read_int_size_string(data, seek);
        self.words       = read_int_size_string(data, seek); //music
        self.author      = self.words.clone(); //GP3
        self.copyright   = read_int_size_string(data, seek);
        self.writer      = read_int_size_string(data, seek); //tabbed by
        self.instructions= read_int_size_string(data, seek); //instructions
        //notices
        let nc = read_int(data, seek) as usize; //notes count
        if nc >0 { for i in 0..nc { self.notice.push(read_int_size_string(data, seek)); println!("  {}\t\t{}",i, self.notice[self.notice.len()-1]); }}
    }

    pub fn read_data(&mut self, data: &Vec<u8>) {
        let mut seek: usize = 0;
        self.read_version(data, &mut seek);
        self.read_meta(data, &mut seek);
        
        if self.version.number < VERSION_5_00 {
            self.triplet_feel = if read_bool(data, &mut seek) {TripletFeel::EIGHTH} else {TripletFeel::NONE};
            //println!("Triplet feel: {}", self.triplet_feel);
            if self.version.number == VERSION_4_0X {} //read lyrics
            self.tempo = read_int(data, &mut seek) as i16;
            self.key.key = read_int(data, &mut seek) as i8;
            println!("Tempo: {} bpm\t\tKey: {}", self.tempo, self.key.to_string());
            if self.version.number == VERSION_4_0X {read_signed_byte(data, &mut seek);} //octave
            self.read_midi_channels(data, &mut seek);
            let measure_count = read_int(data, &mut seek) as usize;
            let track_count = read_int(data, &mut seek) as usize;
            println!("Measures count: {}\tTrack count: {}", measure_count, track_count);
            // Read measure headers. The *measures* are written one after another, their number have been specified previously.
            for i in 1..measure_count + 1  {
                //self.current_measure_number = Some(i as u16);
                self.read_measure_header(data, &mut seek, i);
            }
            //self.current_measure_number = Some(0);
            // read tracks //TODO: FIXME
            for i in 0..track_count {self.read_track(data, &mut seek, i);}
            self.read_measures(data, &mut seek);
            if self.version.number == VERSION_4_0X {} //annotate error reading
        }
        //read GP5 information
        if self.version.number == VERSION_5_00 || self.version.number == VERSION_5_10 {
            //self.lyrics = 
            Lyrics::read(data, &mut seek);
            /*song.masterEffect = self.readRSEMasterEffect()
            song.pageSetup = self.readPageSetup()
            song.tempoName = self.readIntByteSizeString()
            song.tempo = self.readInt()
            song.hideTempo = self.readBool() if self.versionTuple > (5, 0, 0) else False
            song.key = gp.KeySignature((self.readSignedByte(), 0))
            self.readInt()  # octave
            channels = self.readMidiChannels()
            directions = self.readDirections()
            song.masterEffect.reverb = self.readInt()
            measureCount = self.readInt()
            trackCount = self.readInt()
            with self.annotateErrors('reading'):
                self.readMeasureHeaders(song, measureCount, directions)
                self.readTracks(song, trackCount, channels)
                self.readMeasures(song) */
        }
    }

    /// Read all the MIDI channels
    fn read_midi_channels(&mut self, data: &Vec<u8>, seek: &mut usize) { for i in 0u8..64u8 { self.channels.push(MidiChannel::read(data, seek, i)); } }

    /// Read measure header. The first byte is the measure's flags. It lists the data given in the current measure.
    /// 
    /// | **Bit 7** | **Bit 6** | **Bit 5** | **Bit 4** | **Bit 3** | **Bit 2** | **Bit 1** | **Bit 0** |
    /// |-----------|-----------|-----------|-----------|-----------|-----------|-----------|-----------|
    /// | Presence of a double bar  | Tonality of the measure  | Presence of a marker  | Number of alternate ending | End of repeat | Beginning of repeat | Denominator of the (key) signature | Numerator of the (key) signature |
    ///
    /// Each of these elements is present only if the corresponding bit is a 1. The different elements are written (if they are present) from lowest to highest bit.  
    /// Exceptions are made for the double bar and the beginning of repeat whose sole presence is enough, complementary data is not necessary.

    /// * **Numerator of the (key) signature**: `byte`. Numerator of the (key) signature of the piece
    /// * **Denominator of the (key) signature**: `byte`. Denominator of the (key) signature of the piece
    /// * **End of repeat**: `byte`. Number of repeats until the previous Beginning of repeat. Nombre de renvoi jusqu'au début de renvoi précédent.
    /// * **Number of alternate ending**: `byte`. The number of alternate ending.
    /// * **Marker**: The markers are written in two steps:
    /// 1) First is written an `integer` equal to the marker's name length + 1
    /// 2) a string containing the marker's name. Finally the marker's color is written.
    /// * **Tonality of the measure**: `byte`. This value encodes a key (signature) change on the current piece. It is encoded as: `0: C`, `1: G (#)`, `2: D (##)`, `-1: F (b)`, ...
    fn read_measure_header(&mut self, data: &Vec<u8>, seek: &mut usize, number: usize) {
        //println!("N={}\tmeasure_headers={}", number, self.measure_headers.len());
        let flag = read_byte(data, seek);
        let mut mh = MeasureHeader::default();
        mh.number = number as u16;
        mh.start  = 0;
        mh.triplet_feel = self.triplet_feel.clone();
        //we need a previous header for the next 2 flags
        //Numerator of the (key) signature
        if (flag & 0x01 )== 0x01 {mh.time_signature.numerator = read_signed_byte(data, seek);}
        else if number < self.measure_headers.len() {mh.time_signature.numerator = self.measure_headers[number-1].time_signature.numerator;}
        //Denominator of the (key) signature
        if (flag & 0x02) == 0x02 {mh.time_signature.denominator = Duration::read(data, seek, flag);}
        else if number < self.measure_headers.len() {mh.time_signature.denominator = self.measure_headers[number-1].time_signature.denominator.clone();}

        mh.repeat_open = (flag & 0x04) == 0x04; //Beginning of repeat
        if (flag & 0x08) == 0x08 {mh.repeat_close = read_signed_byte(data, seek);} //End of repeat
        if (flag & 0x10) == 0x10 {mh.repeat_alternative = self.read_repeat_alternative(data, seek);} //Number of alternate endin
        if (flag & 0x20) == 0x20 {mh.marker.read(data, seek);} //Presence of a marker
        if (flag & 0x40) == 0x40 { //Tonality of the measure 
            mh.key_signature.key = read_signed_byte(data, seek);
            mh.key_signature.is_minor = read_signed_byte(data, seek) != 0;
        } else if mh.number > 1 && number < self.measure_headers.len() {mh.key_signature = self.measure_headers[number-1].key_signature.clone();}
        mh.double_bar = (flag & 0x80) == 0x80; //presence of a double bar
        self.measure_headers.push(mh);
    }

    /// Read a  track. The first byte is the track's flags. It presides the track's attributes:
    /// 
    /// | **bit 7 to 3** | **bit 2**   | **bit 1**                | **bit 0**   |
    /// |----------------|-------------|--------------------------|-------------|
    /// | Blank bits     | Banjo track | 12 stringed guitar track | Drums track |
    ///
    /// Flags are followed by:
    ///
    /// * **Name**: `string`. A 40 characters long string containing the track's name.
    /// * **Number of strings**: `integer`. An integer equal to the number of strings of the track.
    /// * **Tuning of the strings**: Table of integers. The tuning of the strings is stored as a 7-integers table, the "Number of strings" first integers being really used. The strings are stored from the highest to the lowest.
    /// * **Port**: `integer`. The number of the MIDI port used.
    /// * **Channel**: `integer`. The number of the MIDI channel used. The channel 10 is the drums channel.
    /// * **ChannelE**: `integer`. The number of the MIDI channel used for effects.
    /// * **Number of frets**: `integer`. The number of frets of the instrument.
    /// * **Height of the capo**: `integer`. The number of the fret on which a capo is present. If no capo is used, the value is `0x00000000`.
    /// * **Track's color**: `color`. The track's displayed color in Guitar Pro.
    fn read_track(&mut self, data: &Vec<u8>, seek: &mut usize, _number: usize) {
        let mut track = Track::default();
        //read the flag
        let flags = read_byte(data, seek);
        track.percussion_track = (flags & 0x01) == 0x01; //Drums track
        track.twelve_stringed_guitar_track = (flags & 0x02) == 0x02; //12 stringed guitar track
        track.banjo_track = (flags & 0x04) == 0x04; //Banjo track

        track.name = read_byte_size_string(data, seek);
        *seek += 40 - track.name.len();
        println!("Track: {}", track.name);
        let string_count = read_int(data, seek).to_u8().unwrap();
        track.strings.clear();
        for i in 0i8..7i8 {
            let i_tuning = read_int(data, seek).to_i8().unwrap();
            //println!("tuning: {}", i_tuning);
            if string_count.to_i8().unwrap() > i { track.strings.push((i + 1, i_tuning)); }
        }
        track.port = read_int(data, seek).to_u8().unwrap();
        // Read MIDI channel. MIDI channel in Guitar Pro is represented by two integers. First
        // is zero-based number of channel, second is zero-based number of channel used for effects.
        let index = read_int(data, seek) -1 ;
        let effect_channel = read_int(data, seek) -1;
        if 0 <= index && (index as usize) < self.channels.len() {
            track.channel = self.channels[index as usize].clone();
            if track.channel.get_instrument() < 0 {track.channel.set_instrument(0);}
            if !track.channel.is_percussion_channel() {track.channel.effect_channel = effect_channel.to_u8().unwrap();}
        }
        //
        if track.channel.channel == 9 {track.percussion_track = true;}
        track.fret_count = read_int(data, seek).to_u8().unwrap();
        track.offset = read_int(data, seek);
        track.color = read_color(data, seek);
        //println!("\tInstrument: {} \t Strings: {} {} ({:?})", track.channel.get_instrument_name(), string_count, track.strings.len(), track.strings);
        self.tracks.push(track);
    }

    fn read_repeat_alternative(&mut self, data: &Vec<u8>, seek: &mut usize) -> i8 {
        let value = read_byte(data, seek);
        let mut existing_alternative = 0i8;
        for i in self.measure_headers.len()-1 .. 0 {
            if self.measure_headers[i].repeat_open {break;}
            existing_alternative |= self.measure_headers[i].repeat_alternative;
        }
        return (1 << value) - 1 ^ existing_alternative;
    }

    fn read_measures(&mut self, data: &Vec<u8>, seek: &mut usize) {
        let mut start = DURATION_QUARTER_TIME;
        for h in 0..self.measure_headers.len() {
            self.measure_headers[h].start = start;
            for t in 0..self.tracks.len() {
                self.current_track = Some(self.tracks[t].clone());
                let mut m = Measure::default();
                m.track = self.tracks[t].clone();          //measure = gp.Measure(track, header)
                m.header= self.measure_headers[h].clone(); //self._currentMeasureNumber = measure.number
                { //Read a measure
                    let start = self.measure_headers[h].start;
                    let voice = Voice::default(); //&m.voices[0];
                    let mut current_voice_number = 1;
                    let mut current_beat_number = 1;
                    { //read_voice
                        let beats = read_int(data, seek).to_usize().unwrap();
                        for b in 0..beats {
                            current_beat_number = b + 1
                            //start += self.readBeat(start, voice)
                            //let flags = read_byte(data, seek);
                            //beat = self.getBeat(voice, start)
                        }
                    }
                    //current_voice_number = None
                }
                //track.measures.append(measure)
            }
            start += self.measure_headers[h].length();
        }
        self.current_track = None;
        self.current_measure_number = None;
    }
    /*fn read_measure(&mut self, data: &Vec<u8>, seek: &mut usize) -> Measure {
        //let mut m = Measure::new();
    }*/
    /// The grace notes are stored in the file with 4 variables, written in the following order.
    /// * **Fret**: `byte`. The fret number the grace note is made from.
    /// * **Dynamic**: `byte`. The grace note dynamic is coded like this (default value is 6):
    ///   * 1: ppp
    ///   * 2: pp
    ///   * 3: p
    ///   * 4: mp
    ///   * 5: mf
    ///   * 6: f
    ///   * 7: ff
    ///   * 8: fff
    /// * **Transition**: `byte`. This variable determines the transition type used to make the grace note: `0: None`, `1: Slide`, `2: Bend`, `3: Hammer`.
    /// * **Duration**: `byte`. Determines the grace note duration, coded this way: `3: Sixteenth note`, `2: Twenty-fourth note`, `1: Thirty-second note`.
    fn read_grace_note(&mut self, data: &Vec<u8>, seek: &mut usize) -> GraceEffect {
        let mut ge = GraceEffect::default();
        ge.fret = read_signed_byte(data, seek);
        //TODO: velocity
        //ge.duration = 1 << (7 - read_byte(data, seek));
        ge.duration = match read_byte(data, seek) {
            1 => DURATION_THIRTY_SECOND,
            2 => DURATION_TWENTY_FOURTH, //TODO: FIXME: ?
            3 => DURATION_SIXTEENTH,
            _ => panic!("Cannot get grace note effect duration"),
        };
        ge.is_dead = ge.fret == -1;
        ge.is_on_beat = false;
        ge.transition = match read_signed_byte(data, seek) {
            0 => GraceEffectTransition::None,
            1 => GraceEffectTransition::Slide,
            2 => GraceEffectTransition::Bend,
            3 => GraceEffectTransition::Hammer,
            _ => panic!("Cannot get grace note effect transition"),
        };
        return ge;
    }
}

/// A navigation sign like *Coda* (𝄌: U+1D10C) or *Segno* (𝄋 or 𝄉: U+1D10B or U+1D109).
#[derive(Clone)]
pub enum DirectionSign {
    Coda, Segno,
}

#[derive(Clone)]
pub struct Clipboard {
    pub start_measure: i32,
    pub stop_measure: i32,
    pub start_track: i32,
    pub stop_track: i32,
    pub start_beat: i32,
    pub stop_beat: i32,
    pub sub_bar_copy: bool
}
impl Default for Clipboard {
	fn default() -> Self { Clipboard {start_measure: 1, stop_measure: 1, start_track: 1, stop_track: 1, start_beat: 1, stop_beat: 1, sub_bar_copy: false} }
}

/// An enumeration of different triplet feels.
#[derive(Clone)]
pub enum TripletFeel { NONE, EIGHTH, SIXTEENTH }

#[derive(Clone)]
pub struct MeasureHeader {
    pub number: u16,
	pub start: i64,
	pub time_signature: TimeSignature,
	pub tempo: i32,
	pub marker: Marker,
	pub repeat_open: bool,
	pub repeat_alternative: i8,
	pub repeat_close: i8,
	pub triplet_feel: TripletFeel,
    /// Tonality of the measure
    pub key_signature: KeySignature,
    pub double_bar: bool,
}
impl Default for MeasureHeader {
    fn default() -> Self { MeasureHeader {
        number: 1,
        start: DURATION_QUARTER_TIME,
        tempo: 0,
        repeat_open: false,
        repeat_alternative: 0,
        repeat_close: -1,
        triplet_feel: TripletFeel::NONE,
        key_signature: KeySignature::default(),
        double_bar: false,
        marker: Marker::default(),
        time_signature: TimeSignature {numerator: 4, denominator: Duration::default(), beams: vec![2, 2, 2, 2]}, //TODO: denominator
    }}
}
impl MeasureHeader {
    pub fn length(&self) -> i64 {return (self.time_signature.numerator as i64) * (self.time_signature.denominator.time() as i64);}
    pub fn end(&self) -> i64 {return self.start + self.length();}
}

pub struct _BeatData {
    current_start: i64,
    voices: Vec<VoiceData>
}
/* INIT:
this.currentStart = measure.getStart();
this.voices = new TGVoiceData[TGBeat.MAX_VOICES];
for(int i = 0 ; i < this.voices.length ; i ++ ) this.voices[i] = new TGVoiceData(measure);
*/

pub struct VoiceData {
    start: i64,
    velocity: i32,
    flags: i32,
    //duration: Duration
	duration_value: i32,
	duration_dotted: bool,
	duration_double_dotted: bool,
	//duration_division_type: ?
}

/*impl Default for VoiceData {
    fn default() -> Self { VoiceData {
		flags: 0,
		duration_value: DURATION_QUARTER, duration_dotted: false, duration_double_dotted: false
	}}
}*/
/* DEFAUT: 
this.flags = 0;
this.setStart(measure.getStart());
this.setVelocity(TGVelocities.DEFAULT);
*/

pub const _MAX_STRINGS: i32 = 25;
pub const _MIN_STRINGS: i32 = 1;
pub const _MAX_OFFSET: i32 = 24;
pub const _MIN_OFFSET: i32 = -24;

/// Values of auto-accentuation on the beat found in track RSE settings
#[derive(Clone)]
pub enum Accentuation { None, VerySoft, Soft, Medium, Strong, VeryStrong }

#[derive(Clone)]
pub struct Track {
    pub number: i32,
	pub offset: i32,
	pub channel: MidiChannel, //pub channel_id: i32,
	pub solo: bool,
	pub mute: bool,
    pub visible: bool,
	pub name: String,
    /// A guitar string with a special tuning.
	pub strings: Vec<(i8, i8)>,
	pub color: i32,
    pub percussion_track: bool,
    pub twelve_stringed_guitar_track: bool,
    pub banjo_track: bool,
    pub port: u8,
    pub fret_count: u8,
    pub indicate_tuning: bool,
    pub use_rse: bool,
    pub rse: TrackRse,
}
impl Default for Track {
    fn default() -> Self { Track {
        number: 1,
        offset: 0,
        channel: MidiChannel::default(), //channel_id: 25,
        solo: false, mute: false, visible: true,
        name: String::from("Track 1"),
        strings: vec![(1, 64), (2, 59), (3, 55), (4, 50), (5, 45), (6, 40)],
        banjo_track: false, twelve_stringed_guitar_track: false, percussion_track: false,
        fret_count: 24,
        color: 0xff0000,
        port: 1,
        indicate_tuning: false,
        use_rse: false, rse: TrackRse::default()
    }}
}

/*
this.number = 0;
this.offset = 0;
this.channelId = -1;
this.solo = false;
this.mute = false;
this.name = new String();
this.measures = new ArrayList<TGMeasure>();
this.strings = new ArrayList<TGString>();
this.color = factory.newColor();
this.lyrics = factory.newLyric();
	public void addMeasure(int index,TGMeasure measure){
		measure.setTrack(this);
		this.measures.add(index,measure);
	}
	
	public TGMeasure getMeasure(int index){
		if(index >= 0 && index < countMeasures()){
			return this.measures.get(index);
		}
		return null;
	}
    public String[] getLyricBeats(){
		String lyrics = getLyrics();
		lyrics = lyrics.replaceAll("\n",REGEX); //REGEX = " "
		lyrics = lyrics.replaceAll("\r",REGEX);
		return lyrics.split(REGEX);
	}
*/

pub struct Channel {
    pub id: u16,
	pub bank: u16,
	pub program: u16,
	pub volume: u16,
	pub balance: u16,
	pub chorus: u16,
	pub reverb: u16,
	pub phaser: u16,
	pub tremolo: u16,
	pub name: String,
    /// Channel parameters (key-value)
	pub parameters: HashMap<String, u32>
}
//TODO: handle pub constants
/* 
pub const DEFAULT_PERCUSSION_CHANNEL: i8 = 9;
pub const DEFAULT_PERCUSSION_PROGRAM: i8 = 0;
pub const DEFAULT_PERCUSSION_BANK: i16 = 128;

pub const DEFAULT_BANK: i8 = 0;
pub const DEFAULT_PROGRAM: i8 = 25;
pub const DEFAULT_VOLUME: i8 = 127;
pub const DEFAULT_BALANCE: i8 = 64;
pub const DEFAULT_CHORUS: i8 = 0;
pub const DEFAULT_REVERB: i8 = 0;
pub const DEFAULT_PHASER: i8 = 0;
pub const DEFAULT_TREMOLO: i8 = 0;*/
impl Default for Channel {
    fn default() -> Self { Channel {
        id: 1,
        bank: 0,
        program: 25,
        volume: 127,
        balance: 0,
        chorus: 0,
        reverb: 0,
        phaser: 0,
        tremolo: 0,
        name: String::from("UNDEFINED"),
        parameters: HashMap::new()
    }}
}

/// A marker annotation for beats.
#[derive(Clone)]
pub struct Marker {
    pub title: String,
    pub color: i32,
}
impl Default for Marker {fn default() -> Self { Marker {title: "Section".to_owned(), color: 0xff0000}}}
impl Marker {
    /// Read a marker. The markers are written in two steps:
    /// - first is written an integer equal to the marker's name length + 1
    /// - then a string containing the marker's name. Finally the marker's color is written.
    fn read(&mut self, data: &Vec<u8>, seek: &mut usize) {
        self.title = read_int_size_string(data, seek);
        self.color = read_color(data, seek);
    }
}

/// An enumeration of available clefs
#[derive(Clone)]
pub enum MeasureClef { Treble, Bass, Tenor, Alto }
/// A line break directive: `NONE: no line break`, `BREAK: break line`, `Protect the line from breaking`.
#[derive(Clone)]
pub enum LineBreak { None, Break, Protect }
/// Voice directions indicating the direction of beams
#[derive(Clone,PartialEq)]
pub enum VoiceDirection { None, Up, Down }
/// All beat stroke directions
#[derive(Clone,PartialEq)]
pub enum BeatStrokeDirection { None, Up, Down }
#[derive(Clone)]
pub enum BeatStatus { Empty, Normal, Rest }
/// Characteristic of articulation
#[derive(Clone,PartialEq)]
pub enum SlapEffect { None, Tapping, Slapping, Popping }

/// "A measure contains multiple voices of beats
#[derive(Clone)]
pub struct Measure {
    pub track: Track,
    pub header: MeasureHeader,
    pub clef: MeasureClef,
    /// Max voice count is 2
    pub voices: Vec<Voice>, 
    pub line_break: LineBreak,
}
impl Default for Measure {fn default() -> Self { Measure {track: Track::default(), header: MeasureHeader::default(), clef: MeasureClef::Treble, voices: Vec::with_capacity(2), line_break: LineBreak::None }}}

/// A voice contains multiple beats
#[derive(Clone)]
pub struct Voice {
    //pub measure: Measure, //circular depth?
    pub measure_index: i16,
    pub beats: Vec<Beat>,
    pub directions: VoiceDirection,
}
impl Default for Voice {fn default() -> Self { Voice { measure_index: 0, /*measure: Measure::default(),*/ beats: Vec::new(), directions: VoiceDirection::None }}}

#[derive(Clone)]
pub struct Beat {

}
