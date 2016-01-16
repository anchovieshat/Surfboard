// Based on https://xiph.org/flac/format.html

#![allow(dead_code)]

use std::io::prelude::*;
use std::io;
use byteorder::{LittleEndian, BigEndian, ByteOrder, ReadBytesExt};

#[derive(Debug)]
enum BlockName {
    StreamInfo,
    Padding,
    Application,
    Seektable,
    VorbisComment,
    Picture,
    Other,
}

enum BlockType {
    StreamInfo {
        min_block_size: u16,
        max_block_size: u16,
        min_frame_size: u32,
        max_frame_size: u32,
        sample_rate: u32,
        num_channels: u8,
        bits_per_sample: u8,
        total_samples: u64,
    },
    Padding(u64),
    Application,
    Seektable,
    VorbisComment,
    Picture,
    Other,
}

impl BlockType {
    fn stream<R: io::Read + io::Seek>(r: &mut R) -> BlockType {
        let min_block_size = r.read_u16::<BigEndian>().unwrap();
        let max_block_size = r.read_u16::<BigEndian>().unwrap();

        let mut frame_size_v = Vec::new();
        {
            let mut data_handle = r.take(6);
            data_handle.read_to_end(&mut frame_size_v).unwrap();
        }

        let mut t_max = frame_size_v.split_off(3);
        frame_size_v.reverse();
        t_max.reverse();

        let mut min_frame_size: u32 = 0;
        let mut max_frame_size: u32 = 0;
        for (i, byte) in frame_size_v.iter().enumerate() {
            min_frame_size += (*byte as u32) << (8 * i);
        }
        for (i, byte) in t_max.iter().enumerate() {
            max_frame_size += (*byte as u32) << (8 * i);
        }

        let stream_data = r.read_u64::<BigEndian>().unwrap();
        let sample_rate: u32 = (stream_data >> 44) as u32;
        let num_channels: u8 = (((stream_data << 20) >> 61) + 1) as u8;
        let bits_per_sample: u8 = (((stream_data << 23) >> 59) + 1) as u8;
        println!("min blocksize: {} samples", min_block_size);
        println!("max blocksize: {} samples", max_block_size);
        println!("min framesize: {} bytes", min_frame_size);
        println!("max framesize: {} bytes", max_frame_size);
        println!("sample rate: {}", sample_rate);
        println!("number of channels: {}", num_channels);
        println!("bits per sample: {}", bits_per_sample);

        BlockType::Other
    }

    fn pad<R: io::Read + io::Seek>(r: &mut R) -> BlockType { BlockType::Other }
    fn app<R: io::Read + io::Seek>(r: &mut R) -> BlockType { BlockType::Other }
    fn table<R: io::Read + io::Seek>(r: &mut R) -> BlockType { BlockType::Other }
    fn comment<R: io::Read + io::Seek>(r: &mut R) -> BlockType { BlockType::Other }
    fn picture<R: io::Read + io::Seek>(r: &mut R) -> BlockType { BlockType::Other }
}

struct Block {
    last_meta: bool,
    block_name: BlockName,
    length: u32,
    type_data: Option<BlockType>,
}

impl Block {
    fn parse<R: io::Read + io::Seek>(r: &mut R) -> Block {
        let header = r.read_u32::<BigEndian>().unwrap();

        let last_meta = match header >> 31 {
            0 => false,
            1 => true,
            _ => panic!("wut"),
        };

        let block_name = match (header << 1) >> 24 {
            0 => BlockName::StreamInfo,
            1 => BlockName::Padding,
            2 => BlockName::Application,
            3 => BlockName::Seektable,
            4 => BlockName::VorbisComment,
            5 => BlockName::Picture,
            _ => BlockName::Other,
        };

        let length = (header << 8) >> 8;

        println!("Last block before audio? {}", last_meta);
        println!("block type? {:?}", block_name);
        println!("length: {}", length);

        let type_data = match block_name {
            BlockName::StreamInfo => Some(BlockType::stream(r)),
            BlockName::Padding => Some(BlockType::pad(r)),
            BlockName::Application => Some(BlockType::app(r)),
            BlockName::Seektable => Some(BlockType::table(r)),
            BlockName::VorbisComment => Some(BlockType::comment(r)),
            BlockName::Picture => Some(BlockType::picture(r)),
            BlockName::Other => None,
        };

        Block {
            last_meta: last_meta,
            block_name: block_name,
            length: length,
            type_data: type_data,
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
