#[macro_use]
extern crate anyhow;


use std::convert::{TryFrom, TryInto};

use anyhow::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use resource_mesh_portal_serde::version::v0_0_1::{mesh, PrimitiveFrame};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use std::marker::PhantomData;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}

pub struct FrameWriter<FRAME> where FRAME: TryInto<PrimitiveFrame> {
    stream: PrimitiveFrameWriter,
    phantom: PhantomData<FRAME>
}

impl <FRAME> FrameWriter<FRAME> where FRAME: TryInto<PrimitiveFrame>  {
    pub fn new(stream: PrimitiveFrameWriter) -> Self {
        Self {
            stream,
            phantom: PhantomData
        }
    }
}

impl FrameWriter<mesh::outlet::Frame>  {

    pub async fn write( &mut self, frame: mesh::outlet::Frame ) -> Result<(),Error> {
        let frame = frame.try_into()?;
        self.stream.write(frame).await
    }
}

impl FrameWriter<mesh::inlet::Frame> {

    pub async fn write( &mut self, frame: mesh::inlet::Frame ) -> Result<(),Error> {
        let frame = frame.try_into()?;
        self.stream.write(frame).await
    }
}


pub struct FrameReader<FRAME> where FRAME: TryFrom<PrimitiveFrame> {
    stream: PrimitiveFrameReader,
    phantom: PhantomData<FRAME>
}

impl <FRAME> FrameReader<FRAME>  where FRAME: TryFrom<PrimitiveFrame> {
    pub fn new(stream: PrimitiveFrameReader) -> Self {
        Self {
            stream,
            phantom: PhantomData
        }
    }
}

impl FrameReader<mesh::outlet::Frame> {
    pub async fn read( &mut self ) -> Result<mesh::outlet::Frame,Error> {
        let frame = self.stream.read().await?;
        Ok(mesh::outlet::Frame::try_from(frame)?)
    }
}

impl FrameReader<mesh::inlet::Frame> {
    pub async fn read( &mut self ) -> Result<mesh::inlet::Frame,Error> {
        let frame = self.stream.read().await?;
        Ok(mesh::inlet::Frame::try_from(frame)?)
    }
}

pub struct PrimitiveFrameReader {
    read: OwnedReadHalf
}

impl PrimitiveFrameReader {

    pub fn new(read: OwnedReadHalf ) -> Self {
        Self {
           read
        }
    }

    pub async fn read(&mut self) -> Result<PrimitiveFrame,Error> {
        let size = self.read.read_u32().await? as usize;
        let mut vec: Vec<u8> = Vec::with_capacity(size );
        let buf = vec.as_mut_slice();
        self.read.read(buf).await?;
        Result::Ok(PrimitiveFrame {
            size: size as u32,
            data: vec
        })
    }

    pub async fn read_string(&mut self) -> Result<String,Error> {
        let frame = self.read().await?;
        Ok(frame.try_into()?)
    }


}

pub struct PrimitiveFrameWriter {
    write: OwnedWriteHalf
}

impl PrimitiveFrameWriter {

    pub fn new(write: OwnedWriteHalf) -> Self {
        Self {
            write
        }
    }


    pub async fn write( &mut self, frame: PrimitiveFrame ) -> Result<(),Error> {
        self.write.write_u32(frame.size ).await?;
        self.write.write_all(frame.data.as_slice() ).await?;
        Ok(())
    }


    pub async fn write_string(&mut self, string: String) -> Result<(),Error> {
        let frame = string.into();
        self.write(frame).await
    }
}

