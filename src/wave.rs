#![allow(dead_code)]

use std::str;
use std::io;
use std::io::prelude::*;
use std::collections::HashMap;
use byteorder::{LittleEndian, BigEndian, ByteOrder, ReadBytesExt, WriteBytesExt};

struct Fmt {
    id: u32,
    size: u32,
    audio_fmt: u16,
    num_channels: u16,
    sample_rate: u32,
    byte_rate: u32,
    block_align: u16,
    bits_per_sample: u16,
}

impl Fmt {
    fn write<W: io::Write>(w: &mut W, num_channels: u16, sample_rate: u32, bits_per_sample: u16) {
        let id = BigEndian::read_u32(b" fmt");
        let size = 16; //PCM format size
        let audio_fmt = 1; //Linear Quantization
        let byte_rate = (sample_rate * (num_channels as u32) * (bits_per_sample as u32)) / 8;
        let block_align = (num_channels * bits_per_sample) / 8;

        w.write_u32::<BigEndian>(id).unwrap();
        w.write_u32::<LittleEndian>(size).unwrap();
        w.write_u16::<LittleEndian>(audio_fmt).unwrap();
        w.write_u16::<LittleEndian>(num_channels).unwrap();
        w.write_u32::<LittleEndian>(sample_rate).unwrap();
        w.write_u32::<LittleEndian>(byte_rate).unwrap();
        w.write_u16::<LittleEndian>(block_align).unwrap();
        w.write_u16::<LittleEndian>(bits_per_sample).unwrap();
    }

    fn parse<R: io::Read>(r: &mut R) -> Fmt {
        let id = r.read_u32::<LittleEndian>().unwrap();
        let size = r.read_u32::<LittleEndian>().unwrap();
        let audio_fmt = r.read_u16::<LittleEndian>().unwrap();
        let num_channels = r.read_u16::<LittleEndian>().unwrap();
        let sample_rate = r.read_u32::<LittleEndian>().unwrap();
        let byte_rate = r.read_u32::<LittleEndian>().unwrap();
        let block_align = r.read_u16::<LittleEndian>().unwrap();
        let bits_per_sample = r.read_u16::<LittleEndian>().unwrap();

        let mut t = vec![];
        t.write_u32::<LittleEndian>(id).unwrap();
        println!("\nfmt id: {}", str::from_utf8(&t).unwrap());
        t.clear();

        println!("fmt size: {}", size);
        println!("audio format: {}", audio_fmt);
        println!("number of channels: {}", num_channels);
        println!("sample rate: {} Hz", sample_rate);
        println!("byte rate: {}", byte_rate);
        println!("block alignment: {}", block_align);
        println!("bits per sample: {}", bits_per_sample);

        Fmt {
            id: id,
            size: size,
            audio_fmt: audio_fmt,
            num_channels: num_channels,
            sample_rate: sample_rate,
            byte_rate: byte_rate,
            block_align: block_align,
            bits_per_sample: bits_per_sample,
        }
    }
}

struct Data {
    id: u32,
    size: u32,
    data: Vec<u8>,
}

impl Data {
    fn write<W: io::Write>(w: &mut W, data: Vec<u8>) {
        let id = BigEndian::read_u32(b"data");

        w.write_u32::<BigEndian>(id).unwrap();
        w.write_u32::<LittleEndian>(data.len() as u32).unwrap();
    }

    fn parse<R: io::Read>(r: &mut R) -> Data {
        let id = BigEndian::read_u32(b"data");
        let size = r.read_u32::<LittleEndian>().unwrap();
        let mut data = Vec::new();
        r.read_to_end(&mut data).unwrap();

        println!("data size: {}", size);

        Data {
            id: id,
            size: size,
            data: data,
        }
    }
}

struct Info {
    data: HashMap<String, String>,
}

impl Info {
    fn parse<R: io::Read + io::Seek>(r: &mut R, size: u32) -> Info {
        let mut cur_pos = 4;
        let mut data = HashMap::new();

        while cur_pos < size {
            let mut t = vec![];
            let info_flag = r.read_u32::<LittleEndian>().unwrap();
            let text_size = r.read_u32::<LittleEndian>().unwrap();

            t.write_u32::<LittleEndian>(info_flag).unwrap();
            let info_flag = String::from_utf8(t).unwrap();

            let mut text = String::new();
            {
                let mut str_handle = r.take(text_size as u64);
                str_handle.read_to_string(&mut text).unwrap();

            }

            println!("{}: {}", info_flag, text);

            data.insert(info_flag, text);

            cur_pos += 8 + text_size;

            // Handles word alignment cases
            if cur_pos % 2 != 0 {
                r.seek(io::SeekFrom::Current((cur_pos % 2) as i64)).unwrap();
                cur_pos += cur_pos % 2;
            }
        }
        Info {
            data: data,
        }
    }
}

struct List {
    list_id: u32,
    size: u32,
    type_id: u32,
    info: Option<Info>,
}

impl List {
    fn parse<R: io::Read + io::Seek>(r: &mut R) -> List {
        let list_id = BigEndian::read_u32(b"list");
        let size = r.read_u32::<LittleEndian>().unwrap();

        println!("list size: {}", size);

        let mut t = vec![];

        let type_id = r.read_u32::<LittleEndian>().unwrap();
        t.write_u32::<LittleEndian>(type_id).unwrap();
        let type_string = String::from_utf8(t).unwrap();

        println!("type id: {}", &*type_string);

        let mut info = None;
        if &*type_string == "INFO" {
            info = Some(Info::parse(r, size));
        } else {
            r.seek(io::SeekFrom::Current((size - 4) as i64)).unwrap();
        }

        List {
            list_id: list_id,
            size: size,
            type_id: type_id,
            info: info,
        }
    }
}

pub struct Wave {
    chunk_size: u32,
    format: u32,
    fmt: Fmt,
    list: Option<List>,
    data: Data,
}

impl Wave {
    pub fn write<W: io::Write>(w: &mut W, num_channels: u16, sample_rate: u32, bits_per_sample: u16, data: Vec<u8>) {
        let format = BigEndian::read_u32(b"WAVE");
        let chunk_size = 4 + 16 + (data.len()); // Filesize - (chunk_id + chunk_size)

        w.write_u32::<LittleEndian>(chunk_size as u32).unwrap();
        w.write_u32::<BigEndian>(format).unwrap();

        Fmt::write(w, num_channels, sample_rate, bits_per_sample);
        Data::write(w, data);
    }

    pub fn parse<R: io::Read + io::Seek>(r: &mut R) -> Wave {
        let chunk_size = r.read_u32::<LittleEndian>().unwrap();
        let format = r.read_u32::<LittleEndian>().unwrap();

        println!("size: {}", chunk_size);

        let mut t = vec![];

        t.write_u32::<LittleEndian>(format).unwrap();
        println!("format: {}", str::from_utf8(&t).unwrap());
        t.clear();

        let fmt_chunk = Fmt::parse(r);

        let t_id = r.read_u32::<LittleEndian>().unwrap();
        t.write_u32::<LittleEndian>(t_id).unwrap();
        let mut id = String::from_utf8(t).unwrap();

        let mut list_chunk = None;
        while &*id != "data" {
            match &*id {
                "LIST" => {
                    println!("\nlist id: {}", id);
                    list_chunk = Some(List::parse(r));
                },
                _ => { panic!("Error: cannot parse: {} chunk", id); },
            }

            let t_id = r.read_u32::<LittleEndian>().unwrap();
            let mut tmp = Vec::new();
            tmp.write_u32::<LittleEndian>(t_id).unwrap();
            id = String::from_utf8(tmp).unwrap();
        }

        println!("\ndata id: {}", id);
        let data_chunk = Data::parse(r);

        let song_ms = ((data_chunk.size as u64) * 1000) / fmt_chunk.byte_rate as u64;
        let song_min = song_ms / (60 * 1000);
        let song_sec = song_ms / 1000;
        println!("\nSong length: {}:{:0>2}", song_min, song_sec - (song_min * 60));

        Wave {
            chunk_size: chunk_size,
            format: format,
            fmt: fmt_chunk,
            list: list_chunk,
            data: data_chunk,
        }
    }
}
