#![feature(iterator_try_collect)]
mod data;
#[cfg(test)]

mod lib {
    use std::fs;
    use std::fs::File;
    use std::io::{Cursor, Error, Read};
    use bytestream::ByteOrder::LittleEndian;
    use bytestream::{StreamReader, StreamWriter};
    use sha2::{Sha512,Digest};
    use crate::data::data::VXL;

    #[test]
    fn read_vxl() -> Result<(), Error>{

        let dir = fs::read_dir("vxl")?;
        for file in dir {
            let entry = file?;
            let os_name = entry.file_name();
            let name = os_name.to_str().unwrap();
            if name.ends_with("vxl") {
                println!("Reading [{}]",name);
                get_map_data_from_file(format!("vxl/{}",name))?;
                println!("Read [{}] successfully", name);
            }
        }
        Ok(())
    }

    #[test]
    fn write_vxl() -> Result<(), Error> {

        let dir = fs::read_dir("vxl")?;
        for file in dir {
            let entry = file?;
            let os_name = entry.file_name();
            let name = os_name.to_str().unwrap();
            if name.ends_with("vxl") {
                println!("Reading [{}]",name);
                read_write_map(format!("vxl/{}",name))?;
                println!("Read [{}] successfully", name);
            }
        }

        Ok(())
    }

    fn read_write_map(path: String) -> Result<(), Error> {
        println!("Reading [{}]", path);
        let file = read_file(path)?;
        let mut buffer = Vec::new();

        let mut hasher = Sha512::new();
        let mut hasher2 = Sha512::new();

        hasher.update(&*file.clone());

        let map_data = read_map_data(file)?;

        println!("Generating new VXL from internal data.");
        map_data.write_to(&mut buffer, LittleEndian)?;
        hasher2.update(&*buffer.clone());

        let original_file_hash = hasher.finalize();
        let output_file_hash = hasher2.finalize();

        assert_eq!(original_file_hash[..], output_file_hash[..]);

        println!("Generated files are equal to the original.");

        Ok(())
    }

    fn get_map_data_from_file(path: String) -> Result<VXL, Error> {
        read_map_data(read_file(path)?)
    }

    fn read_file(path: String) -> Result<Vec<u8>, Error> {
        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        Ok(buffer)
    }

    fn read_map_data(buffer: Vec<u8>) -> Result<VXL, Error> {
        let cursor = &mut Cursor::new(buffer);
        VXL::read_from(cursor,  LittleEndian)
    }
}