// Based on https://xiph.org/flac/format.html

#![allow(dead_code)]

use std::io::prelude::*;
use std::io;
use byteorder::{BigEndian, ByteOrder, ReadBytesExt};

#[derive(Debug)]
enum BlockType {
    StreamInfo,
    Padding,
    Application,
    Seektable,
    VorbisComment,
    Picture,
    Other,
}

struct StreamInfo {
    min_block_size: u16,
    max_block_size: u16,
    min_frame_size: u32,
    max_frame_size: u32,
    sample_rate: u32,
    num_channels: u8,
    bits_per_sample: u8,
    total_samples: u64,
}

struct Block {
    last_meta: bool,
    block_type: BlockType,
    length: u32,
    data: Vec<u8>,
}

impl Block {
    fn parse<R: io::Read + io::Seek>(r: &mut R) -> Block {
        let header = r.read_u32::<BigEndian>().unwrap();

        let last_meta = match header >> 31 {
            0 => false,
            1 => true,
            _ => panic!("wut"),
        };

        let block_type = match (header << 1) >> 24 {
            0 => BlockType::StreamInfo,
            1 => BlockType::Padding,
            2 => BlockType::Application,
            3 => BlockType::Seektable,
            4 => BlockType::VorbisComment,
            5 => BlockType::Picture,
            _ => BlockType::Other,
        };

        let length = (header << 8) >> 8;

        println!("Last block before audio? {}", last_meta);
        println!("block type? {:?}", block_type);
        println!("length: {}", length);

        let mut data = Vec::new();
        {
            let mut data_handle = r.take(length as u64);
            data_handle.read_to_end(&mut data).unwrap();
        }

        Block {
            last_meta: last_meta,
            block_type: block_type,
            length: length,
            data: data,
        }
    }
}

pub struct Flac {
    stream_info: Block,
    blocks: Option<Vec<Block>>,
    data: Vec<u8>,
}

impl Flac {
    pub fn parse<R: io::Read + io::Seek>(r: &mut R) -> Flac {
        let stream_info = Block::parse(r);
        let blocks = None;

        let mut data = Vec::new();
        r.read_to_end(&mut data).unwrap();

        Flac {
            stream_info: stream_info,
            blocks: blocks,
            data: data,
        }
    }
}
