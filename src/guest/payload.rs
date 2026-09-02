//! Guest-kernel payload detection and RISC-V Linux Image metadata.
//!
//! PR #51 (`feature/linux-image-loader`) keeps payload-format knowledge behind
//! one interface so VM construction does not need Linux- or ELF-specific
//! branches. A standard RISC-V Linux `Image` is identified by the version 0.2
//! `RSC\x05` header magic and carries its RAM-relative load offset in-header.

const ELF_MAGIC: &[u8; 4] = b"\x7fELF";
const LINUX_IMAGE_HEADER_SIZE: usize = 64;
const LINUX_TEXT_OFFSET: usize = 8;
const LINUX_IMAGE_SIZE: usize = 16;
const LINUX_FLAGS: usize = 24;
const LINUX_MAGIC2: usize = 56;
const LINUX_MAGIC2_BYTES: &[u8; 4] = b"RSC\x05";
const LINUX_BIG_ENDIAN_FLAG: u64 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GuestPayloadError {
    UnsupportedFormat,
    InvalidLinuxImageSize,
    UnsupportedBigEndianLinuxImage,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LinuxImage<'a> {
    bytes: &'a [u8],
    text_offset: u64,
    image_size: u64,
}

impl<'a> LinuxImage<'a> {
    fn parse(bytes: &'a [u8]) -> Result<Self, GuestPayloadError> {
        if bytes.len() < LINUX_IMAGE_HEADER_SIZE {
            return Err(GuestPayloadError::UnsupportedFormat);
        }
        if &bytes[LINUX_MAGIC2..LINUX_MAGIC2 + LINUX_MAGIC2_BYTES.len()] != LINUX_MAGIC2_BYTES {
            return Err(GuestPayloadError::UnsupportedFormat);
        }
        let flags = read_u64(bytes, LINUX_FLAGS);
        if flags & LINUX_BIG_ENDIAN_FLAG != 0 {
            return Err(GuestPayloadError::UnsupportedBigEndianLinuxImage);
        }
        let image_size = read_u64(bytes, LINUX_IMAGE_SIZE);
        // The effective size includes trailing BSS that is intentionally not
        // present in the flat Image file and is cleared by Linux itself.
        if image_size < LINUX_IMAGE_HEADER_SIZE as u64 {
            return Err(GuestPayloadError::InvalidLinuxImageSize);
        }
        Ok(Self {
            bytes,
            text_offset: read_u64(bytes, LINUX_TEXT_OFFSET),
            image_size,
        })
    }

    pub const fn bytes(self) -> &'a [u8] {
        self.bytes
    }

    pub const fn text_offset(self) -> u64 {
        self.text_offset
    }

    pub const fn image_size(self) -> u64 {
        self.image_size
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GuestPayload<'a> {
    Elf(&'a [u8]),
    LinuxImage(LinuxImage<'a>),
}

impl<'a> GuestPayload<'a> {
    /// Detect a supported payload without relying on a build-time format flag.
    /// This lets the same VM construction path boot xv6-rust ELF or Linux Image.
    pub fn detect(bytes: &'a [u8]) -> Result<Self, GuestPayloadError> {
        if bytes.starts_with(ELF_MAGIC) {
            return Ok(Self::Elf(bytes));
        }
        LinuxImage::parse(bytes).map(Self::LinuxImage)
    }
}

fn read_u64(bytes: &[u8], offset: usize) -> u64 {
    let field: [u8; 8] = bytes[offset..offset + 8]
        .try_into()
        .expect("validated Linux Image header field is unavailable");
    u64::from_le_bytes(field)
}

/// PR #51 validates payload discrimination and Linux load metadata at boot.
pub(crate) fn self_test() {
    let elf = [0x7f, b'E', b'L', b'F'];
    assert_eq!(GuestPayload::detect(&elf), Ok(GuestPayload::Elf(&elf)));

    let mut linux = [0u8; LINUX_IMAGE_HEADER_SIZE];
    linux[LINUX_TEXT_OFFSET..LINUX_TEXT_OFFSET + 8].copy_from_slice(&0x20_0000u64.to_le_bytes());
    linux[LINUX_IMAGE_SIZE..LINUX_IMAGE_SIZE + 8]
        .copy_from_slice(&(LINUX_IMAGE_HEADER_SIZE as u64).to_le_bytes());
    linux[LINUX_MAGIC2..LINUX_MAGIC2 + 4].copy_from_slice(LINUX_MAGIC2_BYTES);
    let payload = GuestPayload::detect(&linux).expect("synthetic Linux Image was rejected");
    let GuestPayload::LinuxImage(image) = payload else {
        panic!("Linux Image was detected as ELF");
    };
    assert_eq!(image.text_offset(), 0x20_0000);
    assert_eq!(image.image_size(), LINUX_IMAGE_HEADER_SIZE as u64);
}
