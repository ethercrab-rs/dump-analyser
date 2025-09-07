/// Mailbox header.
///
/// Defined in ETG1000.6 under either `TMBXHEADER` or `MbxHeader` e.g. Table 29 - CoE Elements.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ethercrab_wire::EtherCrabWireReadWrite)]
#[wire(bytes = 8)]
pub struct MailboxHeader {
    /// Mailbox data payload length.
    #[wire(bytes = 2, post_skip_bytes = 2)]
    pub length: u16,
    // /// Address, always zero when master is in control.
    // This field is ignored to save some bytes as it's always zero for EtherCrab's use case.
    // #[wire(bytes = 2)]
    // pub address: u16,
    // reserved6: u8,
    #[wire(pre_skip = 6, bits = 2)]
    pub priority: u8,
    // #[wire(bits = 4)]
    // pub type: u8
    #[wire(bits = 4)]
    pub mailbox_type: MailboxType,
    /// Mailbox counter from 1 to 7 inclusive. Wraps around to 1 when count exceeds 7. 0 is
    /// reserved.
    #[wire(bits = 3)]
    pub counter: u8,
    // _reserved1: u8,
    // _reserved9: u16,
    // _reserved3: u8,
    /// Defined in ETG1000.6 5.6.1 Table 29 – CoE elements.
    #[wire(pre_skip = 13, bits = 4)]
    pub service: CoeService,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ethercrab_wire::EtherCrabWireReadWrite)]
#[repr(u8)]
pub enum MailboxType {
    /// error (ERR)
    Err = 0x00,
    /// ADS over EtherCAT (AoE)
    Aoe = 0x01,
    /// Ethernet over EtherCAT (EoE)
    Eoe = 0x02,
    /// CAN application protocol over EtherCAT (CoE)
    Coe = 0x03,
    /// File Access over EtherCAT (FoE)
    Foe = 0x04,
    /// Servo profile over EtherCAT (SoE)
    Soe = 0x05,
    // 0x06 -0x0e: reserved
    /// Vendor specific
    VendorSpecific = 0x0f,
}

/// Defined in ETG1000.6 Table 29 – CoE elements
#[derive(Clone, Copy, Debug, PartialEq, Eq, ethercrab_wire::EtherCrabWireReadWrite)]
#[repr(u8)]
pub enum CoeService {
    /// Emergency
    Emergency = 0x01,
    /// SDO Request
    SdoRequest = 0x02,
    /// SDO Response
    SdoResponse = 0x03,
    /// TxPDO
    TxPdo = 0x04,
    /// RxPDO
    RxPdo = 0x05,
    /// TxPDO remote request
    TxPdoRemoteRequest = 0x06,
    /// RxPDO remote request
    RxPdoRemoteRequest = 0x07,
    /// SDO Information
    SdoInformation = 0x08,
}
