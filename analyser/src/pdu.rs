//! Types ripped from EtherCrab.
//!
//! Would be nice to import them directly from EtherCrab in the future. See
//! <https://github.com/ethercrab-rs/ethercrab/issues/116>.

use std::time::Duration;

use crate::ETHERCAT_ETHERTYPE;
use ethercrab::{Command, Reads, Writes};
use nom::{
    bytes::complete::take,
    combinator::{map, map_res, verify},
    error::ParseError,
    multi::{fold_many0, many0},
    number::complete::{le_u16, le_u32, u8},
    sequence::pair,
    IResult,
};
use packed_struct::{PackedStruct, PackedStructInfo, PackedStructSlice};
use smoltcp::wire::EthernetFrame;

const LEN_MASK: u16 = 0b0000_0111_1111_1111;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub header: FrameHeader,
    // pub command: Command,
    // pub data: Vec<u8>,
    pub from_master: bool,
    // pub working_counter: u16,
    // pub index: u8,
    pub time: Duration,
    pub wireshark_packet_number: usize,
    pub pdus: Vec<Pdu>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Pdu {
    pub index: u8,
    pub command: Command,
    pub flags: PduFlags,
    pub data: Vec<u8>,
    pub working_counter: u16,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct FrameHeader(pub u16);

impl FrameHeader {
    /// Remove and parse an EtherCAT frame header from the given buffer.
    pub fn parse<'a, E>(i: &'a [u8]) -> IResult<&[u8], Self, E>
    where
        E: ParseError<&'a [u8]>,
    {
        verify(map(nom::number::complete::le_u16, Self), |self_| {
            self_.protocol_type() == ProtocolType::DlPdu
        })(i)
    }

    /// The length of the payload contained in this frame.
    pub fn payload_len(&self) -> u16 {
        self.0 & LEN_MASK
    }

    fn protocol_type(&self) -> ProtocolType {
        let raw = (self.0 >> 12) as u8 & 0b1111;

        raw.into()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, num_enum::FromPrimitive, num_enum::IntoPrimitive)]
#[repr(u8)]
enum ProtocolType {
    DlPdu = 0x01u8,
    NetworkVariables = 0x04,
    Mailbox = 0x05,
    #[num_enum(catch_all)]
    Unknown(u8),
}

/// Parse an EtherCAT PDU from a raw Ethernet II frame.
// Ripped straight out of EtherCrab. Would be nice to expose this as a helper function from
// ethercrab itself eventually.
pub fn parse_pdu(mut raw_packet: EthernetFrame<Vec<u8>>) -> Result<Frame, ethercrab::error::Error> {
    assert_eq!(
        raw_packet.ethertype(),
        ETHERCAT_ETHERTYPE,
        "Not a valid EtherCAT frame"
    );

    let from_master = !raw_packet.src_addr().is_local();

    let i = raw_packet.payload_mut();

    let (i, header) = FrameHeader::parse::<()>(i).expect("FrameHeader");

    // Only take as much as the header says we should
    let (_rest, i) = take::<_, _, ()>(header.payload_len())(i).expect("Body");

    // let mut pdus = Vec::new();

    // let mut i2 = i;

    let (_rest, pdus) = many0(parse_pdu_inner)(i).expect("Bad parse");

    // `_i` should be empty as we `take()`d an exact amount above.
    // debug_assert_eq!(
    //     i.len(),
    //     0,
    //     "{} bytes of trailing data in received frame",
    //     i.len()
    // );

    Ok(Frame {
        header,
        from_master,
        time: Duration::default(),
        wireshark_packet_number: 0,
        pdus,
    })
}

fn parse_pdu_inner(i: &[u8]) -> IResult<&[u8], Pdu> {
    let (i, command_code) = u8(i)?;
    let (i, index) = u8(i)?;

    let (i, command) = parse_command(command_code, i)?;

    let (i, flags) = map_res(take(2usize), PduFlags::unpack_from_slice)(i)?;
    let (i, _irq) = le_u16(i)?;
    let (i, data) = take(flags.length)(i)?;
    let (i, working_counter) = le_u16(i)?;

    Ok((
        i,
        Pdu {
            index,
            command,
            flags,
            data: data.to_vec(),
            working_counter,
        },
    ))
}

#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
pub struct PduFlags {
    /// Data length of this PDU.
    pub(crate) length: u16,
    /// Circulating frame
    ///
    /// 0: Frame is not circulating,
    /// 1: Frame has circulated once
    circulated: bool,
    /// 0: last EtherCAT PDU in EtherCAT frame
    /// 1: EtherCAT PDU in EtherCAT frame follows
    is_not_last: bool,
}

impl PackedStruct for PduFlags {
    type ByteArray = [u8; 2];

    fn pack(&self) -> packed_struct::PackingResult<Self::ByteArray> {
        let raw = self.length & LEN_MASK
            | (self.circulated as u16) << 14
            | (self.is_not_last as u16) << 15;

        Ok(raw.to_le_bytes())
    }

    fn unpack(src: &Self::ByteArray) -> packed_struct::PackingResult<Self> {
        let src = u16::from_le_bytes(*src);

        let length = src & LEN_MASK;
        let circulated = (src >> 14) & 0x01 == 0x01;
        let is_not_last = (src >> 15) & 0x01 == 0x01;

        Ok(Self {
            length,
            circulated,
            is_not_last,
        })
    }
}

impl PackedStructInfo for PduFlags {
    fn packed_bits() -> usize {
        8 * 2
    }
}

const NOP: u8 = 0x00;
const APRD: u8 = 0x01;
const FPRD: u8 = 0x04;
const BRD: u8 = 0x07;
const LRD: u8 = 0x0A;
const BWR: u8 = 0x08;
const APWR: u8 = 0x02;
const FPWR: u8 = 0x05;
const FRMW: u8 = 0x0E;
const LWR: u8 = 0x0B;
const LRW: u8 = 0x0c;

fn parse_command(command_code: u8, i: &[u8]) -> IResult<&[u8], Command> {
    match command_code {
        NOP => Ok((i, Command::Nop)),

        APRD => map(pair(le_u16, le_u16), |(address, register)| {
            Command::Read(Reads::Aprd { address, register })
        })(i),
        FPRD => map(pair(le_u16, le_u16), |(address, register)| {
            Command::Read(Reads::Fprd { address, register })
        })(i),
        BRD => map(pair(le_u16, le_u16), |(address, register)| {
            Command::Read(Reads::Brd { address, register })
        })(i),
        LRD => map(le_u32, |address| Command::Read(Reads::Lrd { address }))(i),

        BWR => map(pair(le_u16, le_u16), |(address, register)| {
            Command::Write(Writes::Bwr { address, register })
        })(i),
        APWR => map(pair(le_u16, le_u16), |(address, register)| {
            Command::Write(Writes::Apwr { address, register })
        })(i),
        FPWR => map(pair(le_u16, le_u16), |(address, register)| {
            Command::Write(Writes::Fpwr { address, register })
        })(i),
        FRMW => map(pair(le_u16, le_u16), |(address, register)| {
            Command::Read(Reads::Frmw { address, register })
        })(i),
        LWR => map(le_u32, |address| Command::Write(Writes::Lwr { address }))(i),

        LRW => map(le_u32, |address| Command::Write(Writes::Lrw { address }))(i),

        other => {
            log::error!("Invalid command code {:#02x}", other);

            Err(nom::Err::Failure(nom::error::Error {
                input: i,
                code: nom::error::ErrorKind::Tag,
            }))
        }
    }
}
