use std::io::{self, Read, Write, Seek, SeekFrom};

use byteorder::{LE, WriteBytesExt};

const AFS_DATA_START: u64 = 0x80000;
const AFS_MAGIC: &[u8; 4] = b"AFS\x00";

#[derive(Clone,Copy,Debug)]
struct AfsFile {
    offset: u32,
    size: u32,
}

#[derive(Clone,Debug)]
pub struct AfsWriter<S, I> {
    inner: S,
    files: I,
}

impl<S, I> AfsWriter<S, I>
    where S: Write + Seek,
          I: IntoIterator,
          I::Item: Read,
          I::IntoIter: ExactSizeIterator,
{
    pub fn new(inner: S, iterable: I) -> AfsWriter<S, I> {
        AfsWriter {
            inner: inner,
            files: iterable,
        }
    }

    // TODO: We can potentially have bad things happen if offset goes past 4GB
    pub fn write(mut self) -> io::Result<()> {
        let file_iter = self.files.into_iter();
        let num_files = file_iter.len();

        let mut offset = AFS_DATA_START;

        // Seek to data start and start writing files
        self.inner.seek(SeekFrom::Start(offset))?;

        let mut file_headers = Vec::new();
        for mut file in file_iter {
            let len = io::copy(&mut file, &mut self.inner)?;

            file_headers.push(AfsFile {
                offset: offset as u32,
                size: len as u32,
            });

            offset += len;

            // Pad to align to 0x800
            if offset % 0x800 != 0 {
                let padding_len = 0x800 - (offset % 0x800);
                for _ in 0..padding_len {
                    self.inner.write_u8(0)?;
                }
                offset += padding_len;
            }
        }

        // Go back to the start and write header info
        self.inner.seek(SeekFrom::Start(0))?;

        // Write afs header
        self.inner.write_all(AFS_MAGIC)?;
        self.inner.write_u32::<LE>(num_files as u32)?;

        for file_header in file_headers {
            self.inner.write_u32::<LE>(file_header.offset)?;
            self.inner.write_u32::<LE>(file_header.size)?;
        }

        Ok(())
    }
}