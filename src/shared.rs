pub mod shared {
    #[derive(Debug, Clone)]
    pub enum WpilogReadErrors {
        FileDoesNotExist,
        ReadError,
        NoDataLeft,
        InvalidHeader,
        InvalidRecoard,
        UnsupportedWpilogVersion,
        MalformedData,
        UseOfEntryIdWithoutStart,
        UseOfEntryIdAfterFinish,
        EntryIdAlreadyStarted,
        SetMetadataWithoutStart,
        FinishWithoutStart,
    }
    pub fn pad_to_n_bytes<const SIZE: usize>(data: Vec<u8>) -> [u8; SIZE] {
        let mut arr: [u8; SIZE] = [0; SIZE];
        for i in 0..SIZE {
            arr[i] = match data.get(i) {
                Some(n) => *n,
                None => 0,
            };
        }
        return arr;
    }

    pub fn next_chunk(
        file: &mut (Vec<u8>, usize),
        size: usize,
    ) -> Result<Vec<u8>, WpilogReadErrors> {
        let mut out: Vec<u8> = Vec::new();
        let mut iter = file.0.iter().skip(file.1);
        for _i in 0..size {
            out.push(
                *(match iter.next() {
                    Some(n) => n,
                    None => return Err(WpilogReadErrors::NoDataLeft),
                }),
            );
        }
        file.1 += size;

        return Ok(out);
    }
}
