use types::{BlobDisperseInfo, BlobLocation};

pub mod types;

pub const ENTRY_SIZE: u32 = 256;
pub const ENTRY_PER_SEGMENT: u32 = 1024;
pub const SEGMENT_SIZE: u32 = ENTRY_SIZE * ENTRY_PER_SEGMENT;
pub const COEFF_SIZE: u32 = 32;
pub const COMMITMENT_SIZE: u32 = 48;

pub fn allocate_rows(blob_disperse_infos: &Vec<BlobDisperseInfo>) -> Vec<BlobLocation> {
    let n = blob_disperse_infos.len();
    let mut locations = vec![
        BlobLocation {
            segment_indexes: vec![],
            offsets: vec![]
        };
        n
    ];
    let mut allocated = vec![0; n];
    let mut segments = 0;
    let mut i = 0;
    while i < n {
        let mut offset = 0;
        let mut j = i;
        while i < n {
            if allocated[j] == blob_disperse_infos[j].rows {
                if j == i {
                    i += 1;
                }
            } else {
                // try to fill one chunk + proof
                let l = blob_disperse_infos[j].cols * COEFF_SIZE + COMMITMENT_SIZE;
                if offset + l <= SEGMENT_SIZE {
                    locations[j].segment_indexes.push(segments);
                    locations[j].offsets.push(offset);
                    allocated[j] += 1;
                    offset += l;
                } else {
                    break;
                }
            }
            // move to next blob
            j += 1;
            if j >= n {
                j = i;
            }
        }
        if offset > 0 {
            segments += 1;
        }
    }
    locations
}
