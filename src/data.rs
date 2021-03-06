pub(crate) mod data {
    use std::collections::{LinkedList, VecDeque};
    use std::io::{Read, Write};
    use bytestream::{ByteOrder, StreamReader, StreamWriter};

    pub(crate) struct VXL {
        cols: Vec<Vec<Column>>
    }

    impl StreamReader for VXL {
        fn read_from<R: Read>(buffer: &mut R, order: ByteOrder) -> std::io::Result<Self> {
            let cols: Vec<Vec<Column>> = (0..512).map(|_| (0..512).map(|_| Column::read_from(buffer, order)).try_collect()).try_collect()?;

            Ok(Self{ cols })
        }
    }

    impl StreamWriter for VXL {
        fn write_to<W: Write>(&self, buffer: &mut W, order: ByteOrder) -> std::io::Result<()> {
            self.cols.iter().map(|it| it.iter().map(|x| x.write_to(buffer, order)).try_collect()).try_collect()
        }
    }

    struct Column {
        data: Vec<Span>
    }

    impl StreamReader for Column {
        fn read_from<R: Read>(buffer: &mut R, order: ByteOrder) -> std::io::Result<Self> {
            let mut data = vec![];
            let mut current_span = Span::read_from(buffer, order)?;

            while current_span.header.length != 0 {
                data.push(current_span);
                current_span = Span::read_from(buffer, order)?;
            }

            data.push(current_span);
            Ok(Self { data })
        }
    }

    impl StreamWriter for Column {
        fn write_to<W: Write>(&self, buffer: &mut W, order: ByteOrder) -> std::io::Result<()> {
            self.data.iter().map(|span| span.write_to(buffer, order)).try_collect()
        }
    }


    #[derive(PartialEq)]
    struct Span {
        header: SpanHeader,
        colors: Vec<BGRAColor>
    }

    impl StreamReader for Span {
        fn read_from<R: Read>(buffer: &mut R, order: ByteOrder) -> std::io::Result<Self> {
            let header = SpanHeader::read_from(buffer, order)?;
            let mut vec = vec![];
            let run;

            if header.length == 0 {
                run = Run::LastSpan {header};
            } else {
                run = Run::Span {header}
            }

            for _ in 0..(run.size() - 1) {
                vec.push(BGRAColor::read_from(buffer,order)?);
            }

            Ok(Self{ header, colors: vec })
        }
    }

    impl StreamWriter for Span {
        fn write_to<W: Write>(&self, buffer: &mut W, order: ByteOrder) -> std::io::Result<()> {
            self.header.write_to(buffer,order)?;

            self.colors.iter().map(|color| color.write_to(buffer,order)).try_collect()?;

            Ok(())
        }
    }

    #[derive(Copy, Clone, PartialEq)]
    struct SpanHeader {
        /// N
        length: u8,
        /// S
        starting_height_tcr: u8,
        /// E
        ending_height_tcr: u8,
        /// A
        starting_height_air: u8
    }

    impl SpanHeader {
        fn get_z(&self) -> u8 {
            return (self.length - 1) - self.get_k();
        }

        fn get_k(&self) -> u8 {
            return (self.ending_height_tcr - self.starting_height_tcr) + 1;
        }

        fn starting_height_solid(&self) -> u8 { self.ending_height_tcr + 1 }

        fn ending_height_air(&self) -> u8 { self.starting_height_tcr - 1 }

        fn length_air(&self) -> u8 {self.starting_height_tcr - self.starting_height_air }
        fn length_tcr (&self) -> u8 { self.get_k() }
        fn length_bcr (&self) -> u8 { self.get_z() }
    }

    impl StreamReader for SpanHeader {
        fn read_from<R: Read>(buffer: &mut R, order: ByteOrder) -> std::io::Result<Self> {
            Ok(Self {
                length: u8::read_from(buffer,order)?,
                starting_height_tcr: u8::read_from(buffer,order)?,
                ending_height_tcr: u8::read_from(buffer,order)?,
                starting_height_air: u8::read_from(buffer,order)?
            })
        }
    }

    impl StreamWriter for SpanHeader {
        fn write_to<W: Write>(&self, buffer: &mut W, order: ByteOrder) -> std::io::Result<()> {
            self.length.write_to(buffer, order)?;
            self.starting_height_tcr.write_to(buffer, order)?;
            self.ending_height_tcr.write_to(buffer,order)?;
            self.starting_height_air.write_to(buffer, order)?;

            Ok(())
        }
    }

    #[derive(PartialEq, Copy, Clone)]
    struct BGRAColor {
        b: u8,
        g: u8,
        r: u8,
        a: u8
    }

    impl StreamReader for BGRAColor {
        fn read_from<R: Read>(buffer: &mut R, order: ByteOrder) -> std::io::Result<Self> {
            Ok(Self {
                b: u8::read_from(buffer,order)?,
                g: u8::read_from(buffer,order)?,
                r: u8::read_from(buffer,order)?,
                a: u8::read_from(buffer,order)?
            })
        }
    }

    impl StreamWriter for BGRAColor {
        fn write_to<W: Write>(&self, buffer: &mut W, order: ByteOrder) -> std::io::Result<()> {
            self.b.write_to(buffer,order)?;
            self.g.write_to(buffer,order)?;
            self.r.write_to(buffer,order)?;
            self.a.write_to(buffer,order)?;

            Ok(())
        }
    }

    /// Internal representation used for reading a "Run" of voxel data.
    /// Equivalent to `Span`
    enum Run {
        LastSpan {
            header: SpanHeader
        },
        Span {
            header: SpanHeader
        }
    }

    impl Run {
        fn size(&self) -> u8 {
            return match self {
                Run::LastSpan { header, .. } => { 1 + header.get_k() }
                Run::Span { header, .. } => { header.length }
            }
        }
    }

    #[derive(Copy, Clone)]
    enum Voxel {
        Open,
        Colored {color: BGRAColor},
        /// Has `color` due to **Surface Voxel Rule**
        Solid {color: BGRAColor}
    }

    impl Column {
        fn starting_height_bcr(&self, current_span_idx: usize) -> Option<u8> {
            let current_span = self.data.get(current_span_idx)?;
            let m = self.get_m(current_span_idx + 1)?;
            Some(m - current_span.header.get_z())
        }

        fn length_solid(&self, current_span_idx: usize) -> Option<u8> {
            let current_span = self.data.get(current_span_idx)?;
            let m = self.get_m(current_span_idx)?;
            Some(m - current_span.header.get_z() - current_span.header.starting_height_solid())
        }

        fn ending_height_bcr(&self, current_span_idx: usize) -> Option<u8> {
            let m = self.get_m(current_span_idx)?;
            Some(m + 1)
        }

        fn ending_height_solid(&self, current_span_idx: usize) -> Option<u8> {
            let current_span = self.data.get(current_span_idx)?;
            let m = self.get_m(current_span_idx)?;
            Some(m - current_span.header.get_z() - 1)
        }

        fn get_m(&self, current_span_idx: usize) -> Option<u8> {
            if self.data.get(current_span_idx)?.header.length == 0 {
                return Some(64)
            }

            let next = &self.data.get(current_span_idx + 1)?;

            Some(next.header.starting_height_air)
        }
    }
}
