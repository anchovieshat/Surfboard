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

#[derive(Debug, PartialEq)]
enum BlockStrategy {
    FixedBlocksize,
    VariableBlocksize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Channels {
    Mono,
    LR,
    LRC,
    FlFrBlBr,
    FlFrFcBlBr,
    FlFrFcLFEBlBr,
    FlFrFcLFEBcSlSr,
    FlFrFcLFEBlBrSlSr,
    LS,
    SR,
    MS,
}

impl Channels {
    fn channel_num(c: Channels) -> u8 {
        match c {
            Channels::Mono => 1,
            Channels::LR => 2,
            Channels::LRC => 3,
            Channels::FlFrBlBr => 4,
            Channels::FlFrFcBlBr => 5,
            Channels::FlFrFcLFEBlBr => 6,
            Channels::FlFrFcLFEBcSlSr => 7,
            Channels::FlFrFcLFEBlBrSlSr => 8,
            Channels::LS => 2,
            Channels::SR => 2,
            Channels::MS => 2,
        }
    }
}

// ref: http://permalink.gmane.org/gmane.comp.audio.compression.flac.devel/3033
fn decode_utf8_val<R: io::Read + io::Seek>(r: &mut R) -> u64 {
    let tx = 0x80;
    let t2 = 0xC0;
    let t3 = 0xE0;
    let t4 = 0xF0;
    let t5 = 0xF8;
    let t6 = 0xFC;
    let t7 = 0xFE;
    let t8 = 0xFF;

    let maskx = 0x3F;
    let mask2 = 0x1F;
    let mask3 = 0x0F;
    let mask4 = 0x07;
    let mask5 = 0x03;
    let mask6 = 0x01;

    let rune1_max = (1 << 7) - 1;
    let rune2_max = (1 << 11) - 1;
    let rune3_max = (1 << 16) - 1;
    let rune4_max = (1 << 21) - 1;
    let rune5_max = (1 << 26) - 1;
    let rune6_max: u32 = (1 << 31) - 1;

    let c0 = r.read_u8().unwrap();

    let mut l = 0; // leading bits - 1
    let mut n = 0;

    if c0 < tx { return c0 as u64 }
    if c0 < t2 { panic!("unexpected continuation byte") }

    if c0 < t3 { l = 1; n = (c0 as u64) & mask2; }
    else if c0 < t4 { l = 2; n = (c0 as u64) & mask3; }
    else if c0 < t5 { l = 3; n = (c0 as u64) & mask4; }
    else if c0 < t6 { l = 4; n = (c0 as u64) & mask5; }
    else if c0 < t7 { l = 5; n = (c0 as u64) & mask6; }
    else if c0 < t8 { l = 6; n = 0; }

    for _ in  0..l {
        n <<= 6;
        let c = r.read_u8().unwrap();

        if c < tx || t2 <= c { panic!("expected continuation byte!") }

        n |= (c as u64) & maskx;
    }

    if l <= rune1_max { panic!("larger number than necessary") }
    if l <= rune2_max { panic!("larger number than necessary") }
    if l <= rune3_max { panic!("larger number than necessary") }
    if l <= rune4_max { panic!("larger number than necessary") }
    if l <= rune5_max { panic!("larger number than necessary") }
    if l <= rune6_max { panic!("larger number than necessary") }

    n
}

struct FrameHeader {
    sync_code: u16,
    block_strategy: BlockStrategy,
    block_size: u16,
    sample_rate: u32,
    channel_val: Channels,
    sample_size: u8,
    crc_8: u8,
}

impl FrameHeader {
    pub fn parse<R: io::Read + io::Seek>(r: &mut R, rate: u32, bps: u8) -> FrameHeader {
        let header = r.read_u32::<BigEndian>().unwrap();

        let sync_code = (header >> 18) as u16;
        let block_strategy = match (header << 19) >> 31 {
            0 => BlockStrategy::FixedBlocksize,
            1 => BlockStrategy::VariableBlocksize,
            _ => panic!("wut"),
        };

        let block_size_bits =  (header << 16) >> 28;
        let sample_rate_bits = (header << 20) >> 28;
        let channel_val_bits = (header << 24) >> 28;
        let sample_size_bits = (header << 27) >> 29;

        let sample_rate = match sample_rate_bits {
            0 => rate,
            1 => 88200,
            2 => 176400,
            3 => 192000,
            4 => 8000,
            5 => 16000,
            6 => 22050,
            7 => 24000,
            8 => 32000,
            9 => 44100,
            10 => 48000,
            11 => 96000,
            15 => panic!("invalid rate!"),
            _ => 0,
        };

        let channel_val = match channel_val_bits {
            0 => Channels::Mono,
            1 => Channels::LR,
            2 => Channels::LRC,
            3 => Channels::FlFrBlBr,
            4 => Channels::FlFrFcBlBr,
            5 => Channels::FlFrFcLFEBlBr,
            6 => Channels::FlFrFcLFEBcSlSr,
            7 => Channels::FlFrFcLFEBlBrSlSr,
            8 => Channels::LS,
            9 => Channels::SR,
            10 => Channels::MS,
            _ => panic!("invalid channel setting!"),
        };

        let sample_size = match sample_size_bits {
            0 => bps,
            1 => 8,
            2 => 12,
            4 => 16,
            5 => 20,
            6 => 24,
            _ => panic!("incorrect sample size!"),
        };

        let mut frame_num = 0;
        let mut sample_num = 0;
        if block_strategy == BlockStrategy::FixedBlocksize {
            frame_num = decode_utf8_val(r);
        } else {
            sample_num = decode_utf8_val(r);
        }

        let block_size = match block_size_bits {
            1 => 192,
            2 ... 5 => (576 * (1 << (block_size_bits - 2))),
            6 => { (r.read_u8().unwrap() + 1) as u16 },
            7 => { r.read_u16::<LittleEndian>().unwrap() + 1 },
            8 ... 15 => (256 * (1 << (block_size_bits - 8))),
            _ => panic!("incorrect block size!"),
        };

        let crc_8 = r.read_u8().unwrap();

        println!("\nsync code: {:0>8b}", sync_code);
        println!("Frame number: {}", frame_num);
        println!("Sample number: {}", sample_num);
        println!("block strategy: {:?}", block_strategy);
        println!("block size: {}", block_size);
        println!("sample rate: {}", sample_rate);
        println!("channel assignment: {:?}", channel_val);
        println!("sample size: {}", sample_size);
        println!("CRC-8: {}", crc_8);

        FrameHeader {
            sync_code: sync_code,
            block_strategy: block_strategy,
            block_size: block_size,
            sample_rate: sample_rate,
            channel_val: channel_val,
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
}

struct Subframe {
    sub_type: SubframeType,
    wasted_bits_per_sample: bool,
    order: u32,
    samples: Vec<u32>,
}

// Will require further thinking...
/*fn signExtend(x: BigUint, n: u32) -> i32 {
	if x & (1 << (n - 1)) != 0 {
		// Sign extend x.
		return (x | (!0 << n)) as i32
	}
	x as i32
}*/

impl Subframe {
    pub fn parse<R: io::Read + io::Seek>(r: &mut R, bps: u8, block_size: u16) -> Subframe {
        let header = r.read_u8().unwrap();

        println!("\nheader: {:0>8b}", header);

        if (header >> 7) != 0 {
            panic!("non-zero padding!");
        }

        let sub_type_bits = ((header as u32) << 1) >> 2;
        let mut order = 0;
        let sub_type = match sub_type_bits {
            0 => SubframeType::Constant,
            1 => SubframeType::Verbatim,
            8 ... 12 => { order = sub_type_bits & 0x07; SubframeType::Fixed },
            32 ... 63 => { order = (sub_type_bits & 0x1F) + 1; SubframeType::LPC },
            _ => panic!("Unknown subframe type!"),
        };

        let wasted_bits_per_sample = match (header << 7) >> 7 {
            0 => false,
            1 => true,
            _ => panic!("wut"),
        };

        println!("subframe type: {:?}", sub_type);
        println!("order: {}", order);
        println!("wasted_bits_per_sample: {}", wasted_bits_per_sample);

        let samples = Subframe::decode_samples(r, &sub_type, order, block_size, bps);

        Subframe {
            sub_type: sub_type,
            wasted_bits_per_sample: wasted_bits_per_sample,
            order: order,
            samples: samples,
        }
    }

    fn decode_samples<R: io::Read + io::Seek>(r: &mut R, sub_type: &SubframeType, order: u32, block_size: u16, bps: u8) -> Vec<u32> {
        let samples = Vec::with_capacity(block_size as usize);
        println!("bits per sample: {}", bps);

        for _ in 0..order {
            let mut warm_up_data_v = Vec::new();
            {
                let mut data_handle = r.take(bps as u64);
                data_handle.read_to_end(&mut warm_up_data_v).unwrap();
            }

            let warm_up_data = BigUint::from_bytes_be(&*warm_up_data_v);
            println!("warm up values: {:b}", warm_up_data);
        }

        samples
    }
}

struct Frame {
    header: FrameHeader,
    subframes: Vec<Subframe>,
    footer: u16,
}

impl Frame {
    pub fn parse<R: io::Read + io::Seek>(r: &mut R, sample_rate: u32, sample_size: u8) -> Frame {
        let header = FrameHeader::parse(r, sample_rate, sample_size);

        let bps = header.sample_size;
        let block_size = header.block_size;

        let mut subframes = Vec::new();
        let mut subframe: Option<Subframe> = None;
        for channel in 0..Channels::channel_num(header.channel_val) {
            let mut t_bps = bps;
            match header.channel_val {
                Channels::SR => { if channel == 0 { t_bps += 1; } },
                Channels::LS => { if channel == 1 { t_bps += 1; } },
                _ => (),
            }
            subframe = Some(Subframe::parse(r, t_bps, block_size));
            subframes.push(subframe.unwrap());
        }

        let footer = r.read_u16::<LittleEndian>().unwrap();
        println!("footer CRC-16: {}", footer);

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

        // There must be a better way to do this!
        let (sample_rate, size) = match stream_info.clone().type_data.unwrap() {
            BlockType::StreamInfo {
                min_block_size,
                max_block_size,
                min_frame_size,
                max_frame_size,
                sample_rate,
                num_channels,
                bits_per_sample,
                total_samples,
                sig,
            } => (sample_rate, bits_per_sample),
            _ => panic!("Not StreamInfo?"),
        };

        let mut frames = Vec::new();
        let frame = Frame::parse(r, sample_rate, size);
        frames.push(frame);

        Flac {
            stream_info: stream_info,
            blocks: blocks,
            frames: frames,
        }
    }
}
