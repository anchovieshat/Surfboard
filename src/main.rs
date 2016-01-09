extern crate byteorder;
extern crate rustc_serialize;
extern crate bincode;

use std::str;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::env;
use byteorder::{LittleEndian, BigEndian, ByteOrder, ReadBytesExt, WriteBytesExt};

#[derive(RustcEncodable)]
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
    fn new(num_channels: u16, sample_rate: u32, bits_per_sample: u16) -> Fmt {
        let id = BigEndian::read_u32(b"fmt ");
        let size = 16; //PCM format size
        let audio_fmt = 1; //Linear Quantization
        let byte_rate = (sample_rate * (num_channels as u32) * (bits_per_sample as u32)) / 8;
        let block_align = (num_channels * bits_per_sample) / 8;

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

#[derive(RustcEncodable)]
struct Data {
    id: u32,
    size: u32,
    data: Vec<u8>,
}

impl Data {
    fn new(data: Vec<u8>) -> Data {
        let id = BigEndian::read_u32(b"data");
        Data {
            id: id,
            size: data.len() as u32,
            data: data,
        }
    }

    fn parse<R: io::Read>(r: &mut R) {
        let id = r.read_u32::<LittleEndian>().unwrap();
        let size = r.read_u32::<LittleEndian>().unwrap();
        let mut data = Vec::new();
        r.read_to_end(&mut data);

        let mut t = vec![];
        t.write_u32::<LittleEndian>(id).unwrap();
        println!("\ndata id: {}", str::from_utf8(&t).unwrap());
        t.clear();

        println!("data size: {}", size);

    }
}

struct List {
    list_id: u32,
    size: u32,
    type_id: u32,
}

impl List {
    fn parse<R: io::Read>(r: &mut R) {
        let list_id = r.read_u32::<LittleEndian>().unwrap();
        let size = r.read_u32::<LittleEndian>().unwrap();
        let type_id = r.read_u32::<LittleEndian>().unwrap();
        let new_id = r.read_u32::<LittleEndian>().unwrap();


        let mut t = vec![];
        t.write_u32::<LittleEndian>(list_id).unwrap();
        println!("\nlist id: {}", str::from_utf8(&t).unwrap());
        t.clear();

        println!("list size: {}", size);

        t.write_u32::<LittleEndian>(type_id).unwrap();
        println!("type id: {}", str::from_utf8(&t).unwrap());
        t.clear();
    }
}

#[derive(RustcEncodable)]
struct Wave {
    chunk_id: u32,
    chunk_size: u32,
    format: u32,
    fmt: Fmt,
    data: Data,
}

impl Wave {
    fn new(num_channels: u16, sample_rate: u32, bits_per_sample: u16, data: Vec<u8>) -> Wave {
        let chunk_id = BigEndian::read_u32(b"RIFF");
        let format = BigEndian::read_u32(b"WAVE");
        let fmt_chunk = Fmt::new(num_channels, sample_rate, bits_per_sample);
        let data_chunk = Data::new(data);
        let chunk_size = 4 + (8 + fmt_chunk.size) + (8 + data_chunk.size); // Filesize - (chunk_id + chunk_size)

        Wave {
            chunk_id: chunk_id,
            chunk_size: chunk_size as u32,
            format: format,
            fmt: fmt_chunk,
            data: data_chunk,
        }
    }

    fn parse<R: io::Read + io::Seek>(r: &mut R) {
        r.seek(io::SeekFrom::Start(0)).unwrap();
        let chunk_id = r.read_u32::<LittleEndian>().unwrap();
        let chunk_size = r.read_u32::<LittleEndian>().unwrap();
        let format = r.read_u32::<LittleEndian>().unwrap();

        let mut t = vec![];
        t.write_u32::<LittleEndian>(chunk_id).unwrap();
        println!("identifier: {}", str::from_utf8(&t).unwrap());
        t.clear();

        println!("size: {}", chunk_size);

        t.write_u32::<LittleEndian>(format).unwrap();
        println!("format: {}", str::from_utf8(&t).unwrap());
        t.clear();

        let fmt_chunk = Fmt::parse(r);
        let list_chunk = List::parse(r);

        /*Wave {
            chunk_id: chunk_id,
            chunk_size: chunk_size,
            format: format,
            fmt: fmt_chunk,
            data: data_chunk,
        }*/
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    //let mut wav_file = File::create(&args[1]).unwrap();
    let mut base_file = File::open(&args[1]).unwrap();

    let wav = Wave::parse(&mut base_file);

    //let wav = Wave::new(1, 44100, 8, data);

    //bincode::rustc_serialize::encode_into(&wav, &mut wav_file, bincode::SizeLimit::Infinite).unwrap();
}
