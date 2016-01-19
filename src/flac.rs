// Based on https://xiph.org/flac/format.html

#![allow(dead_code)]

use std::io::prelude::*;
use std::io;
use num::bigint::BigUint;
use byteorder::{LittleEndian, BigEndian, ByteOrder, ReadBytesExt};

#[derive(Debug, Clone)]
enum BlockName {
    StreamInfo,
    Padding,
    Application,
    Seektable,
    VorbisComment,
    Picture,
    Other,
}

#[derive(Debug, Clone)]
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
        sig: BigUint,
    },
    Padding(u64),
    Application,
    Seektable,
    VorbisComment {
        vendor_string: String,
        comments: Vec<String>,
    },
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
        let sample_rate: u32 = (stream_data >> 44) as u32; // 20 bits
        let num_channels: u8 = (((stream_data << 20) >> 61) + 1) as u8; // 3 bits
        let bits_per_sample: u8 = (((stream_data << 23) >> 59) + 1) as u8; // 5 bits
        let total_samples: u64 = ((stream_data << 36) >> 36) as u64; //36 bits

        let mut sig_v = Vec::new();
        {
            let mut data_handle = r.take(16);
            data_handle.read_to_end(&mut sig_v).unwrap();
        }

        let sig = BigUint::from_bytes_be(&*sig_v);

        println!("min blocksize: {} samples", min_block_size);
        println!("max blocksize: {} samples", max_block_size);
        println!("min framesize: {} bytes", min_frame_size);
        println!("max framesize: {} bytes", max_frame_size);
        println!("sample rate: {}", sample_rate);
        println!("number of channels: {}", num_channels);
        println!("bits per sample: {}", bits_per_sample);
        println!("total samples: {}", total_samples);
        println!("md5 sig: {:x}", sig);

        BlockType::StreamInfo {
            min_block_size: min_block_size,
            max_block_size: max_block_size,
            min_frame_size: min_frame_size,
            max_frame_size: max_frame_size,
            sample_rate: sample_rate,
            num_channels: num_channels,
            bits_per_sample: bits_per_sample,
            total_samples: total_samples,
            sig: sig,
        }
    }

    fn pad<R: io::Read + io::Seek>(r: &mut R) -> BlockType { BlockType::Other }
    fn app<R: io::Read + io::Seek>(r: &mut R) -> BlockType { BlockType::Other }
    fn table<R: io::Read + io::Seek>(r: &mut R) -> BlockType { BlockType::Other }
    fn comment<R: io::Read + io::Seek>(r: &mut R) -> BlockType {
        let vendor_length = r.read_u32::<LittleEndian>().unwrap();
        let mut vendor_string = String::new();
        {
            let mut string_handle = r.take(vendor_length as u64);
            string_handle.read_to_string(&mut vendor_string).unwrap();
        }

        let comment_list_length = r.read_u32::<LittleEndian>().unwrap();

        let mut comment_list = Vec::new();
        for i in 0..comment_list_length {
            let comment_length = r.read_u32::<LittleEndian>().unwrap();
            let mut comment_string = String::new();
            {
                let mut string_handle = r.take(comment_length as u64);
                string_handle.read_to_string(&mut comment_string).unwrap();
            }

            comment_list.push(comment_string);
        }

        println!("vendor string: {}", vendor_string);
        println!("comment list: {:?}", comment_list);

        BlockType::VorbisComment {
            vendor_string: vendor_string,
            comments: comment_list,
        }
    }
    fn picture<R: io::Read + io::Seek>(r: &mut R) -> BlockType { BlockType::Other }
}

#[derive(Debug, Clone)]
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

        let block_name = match (header << 1) >> 25 {
            0 => BlockName::StreamInfo,
            1 => BlockName::Padding,
            2 => BlockName::Application,
            3 => BlockName::Seektable,
            4 => BlockName::VorbisComment,
            5 => BlockName::Picture,
            _ => BlockName::Other,
        };

        let length = (header << 8) >> 8;

        println!("\nLast block before audio? {}", last_meta);
        println!("{:?}", block_name);
        println!("-----------------------------");

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

#[derive(Debug)]
enum BlockStrategy {
    FixedBlocksize,
    VariableBlocksize,
}

struct FrameHeader {
    sync_code: u16,
    block_strategy: BlockStrategy,
    block_size: u8,
    sample_rate: u8,
    channel_assignment: u8,
    sample_size: u8,
    crc_8: u8,
}

impl FrameHeader {
    pub fn parse<R: io::Read + io::Seek>(r: &mut R) -> FrameHeader {
        let header = r.read_u32::<BigEndian>().unwrap();

        let sync_code = (header >> 18) as u16;
        let block_strategy = match (header << 19) >> 31 {
            0 => BlockStrategy::FixedBlocksize,
            1 => BlockStrategy::VariableBlocksize,
            _ => panic!("wut"),
        };

        let block_size = ((header << 16) >> 28) as u8;
        let sample_rate = ((header << 20) >> 28) as u8;
        let channel_assignment = ((header << 24) >> 28) as u8;
        let sample_size = ((header << 27) >> 29) as u8;
        let crc_8 = r.read_u8().unwrap();

        println!("\nsync code: {:0>8b}", sync_code);
        println!("block strategy: {:?}", block_strategy);
        println!("block size: {:0>4b}", block_size);
        println!("sample rate: {:0>4b}", sample_rate);
        println!("channel assignment: {:0>4b}", channel_assignment);
        println!("sample size: {:0>3b}", sample_size);
        println!("CRC-8: {}", crc_8);

        FrameHeader {
            sync_code: sync_code,
            block_strategy: block_strategy,
            block_size: block_size,
            sample_rate: sample_rate,
            channel_assignment: channel_assignment,
            sample_size: sample_size,
            crc_8: crc_8,
        }
    }
}

#[derive(Debug)]
enum SubframeType {
    Constant,
    Verbatim,
    Fixed,
    LPC,
    Reserved,
}

struct Subframe {
    sub_type: SubframeType,
    wasted_bits_per_sample: bool,
}

impl Subframe {
    pub fn parse<R: io::Read + io::Seek>(r: &mut R) -> Subframe {
        let header = r.read_u8().unwrap();

        let sub_type = match (header << 2) >> 2 {
            0 => SubframeType::Constant,
            1 => SubframeType::Verbatim,
            8 ... 12 => SubframeType::Fixed,
            32 ... 63 => SubframeType::LPC,
            _ => SubframeType::Reserved,
        };

        let wasted_bits_per_sample = match (header << 7) >> 7 {
            0 => false,
            1 => true,
            _ => panic!("wut"),
        };

        println!("\nheader: {:0>8b}", header);
        println!("subframe type: {:?}", sub_type);
        println!("wasted_bits_per_sample: {}", wasted_bits_per_sample);

        Subframe {
            sub_type: sub_type,
            wasted_bits_per_sample: wasted_bits_per_sample,
        }
    }
}

struct Frame {
    header: FrameHeader,
    subframes: Vec<Subframe>,
    footer: u16,
}

impl Frame {
    pub fn parse<R: io::Read + io::Seek>(r: &mut R) -> Frame {
        let header = FrameHeader::parse(r);
        let mut subframes = Vec::new();
        let subframe = Subframe::parse(r);
        subframes.push(subframe);
        let footer = r.read_u16::<LittleEndian>().unwrap();

        Frame {
            header: header,
            subframes: subframes,
            footer: footer,
        }
    }
}

pub struct Flac {
    stream_info: Block,
    blocks: Option<Vec<Block>>,
    frames: Vec<Frame>,
}

impl Flac {
    pub fn parse<R: io::Read + io::Seek>(r: &mut R) -> Flac {
        let stream_info = Block::parse(r);
        let mut block_list = Vec::new();

        let mut blocks = None;

        if stream_info.last_meta == false {
            let mut t_block = Block::parse(r);
            block_list.push(t_block.clone());

            while t_block.last_meta == false {
                block_list.push(t_block);
                t_block = Block::parse(r);
            }

            blocks = Some(block_list);
        }

        let mut frames = Vec::new();
        let frame = Frame::parse(r);
        frames.push(frame);

        Flac {
            stream_info: stream_info,
            blocks: blocks,
            frames: frames,
        }
    }
}
