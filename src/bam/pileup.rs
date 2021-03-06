// Copyright 2014 Johannes Köster.
// Licensed under the MIT license (http://opensource.org/licenses/MIT)
// This file may not be copied, modified, or distributed
// except according to those terms.

use std::slice;
use std::iter;

use htslib;

use bam::record;


/// Iterator over alignments of a pileup.
pub type Alignments<'a> = iter::Map<
    slice::Iter<'a, htslib::bam_pileup1_t>,
    fn(&'a htslib::bam_pileup1_t) -> Alignment<'a>
>;


/// A pileup over one genomic position.
pub struct Pileup {
    inner: *const htslib::bam_pileup1_t,
    depth: u32,
    tid: u32,
    pos: u32,
}


impl Pileup {
    pub fn tid(&self) -> u32 {
        self.tid
    }

    pub fn pos(&self) -> u32 {
        self.pos
    }

    pub fn depth(&self) -> u32 {
        self.depth
    }

    pub fn alignments(&self) -> Alignments {
        self.inner().iter().map(Alignment::new)
    }

    fn inner(&self) -> &[htslib::bam_pileup1_t] {
        unsafe { slice::from_raw_parts(self.inner as *mut htslib::bam_pileup1_t, self.depth as usize) }
    }
}


/// An aligned read in a pileup.
pub struct Alignment<'a> {
    inner: &'a htslib::bam_pileup1_t,
}


impl<'a> Alignment<'a> {
    pub fn new(inner: &'a htslib::bam_pileup1_t) -> Self {
        Alignment { inner: inner }
    }

    /// Position within the read.
    pub fn qpos(&self) -> usize {
        self.inner.qpos as usize
    }

    /// Insertion, deletion (with length) or None if no indel.
    pub fn indel(&self) -> Indel {
        match self.inner.indel {
            len if len < 0 => Indel::Del(-len as u32),
            len if len > 0 => Indel::Ins(len as u32),
            _              => Indel::None
        }
    }

    /// The corresponding record.
    pub fn record(&self) -> record::Record {
        record::Record::from_inner(self.inner.b)
    }
}


#[derive(PartialEq)]
#[derive(Debug)]
pub enum Indel {
    Ins(u32),
    Del(u32),
    None
}


/// Iterator over pileups.
pub struct Pileups {
    itr: htslib::bam_plp_t,
}


impl Pileups {
    pub fn new(itr: htslib::bam_plp_t) -> Self {
        Pileups { itr: itr }
    }

    pub fn set_max_depth(&mut self, depth: u32) {
        unsafe { htslib::bam_plp_set_maxcnt(self.itr, depth as i32); }
    }
}


impl Iterator for Pileups {
    type Item = Result<Pileup, PileupError>;

    fn next(&mut self) -> Option<Result<Pileup, PileupError>> {
        let (mut tid, mut pos, mut depth) = (0i32, 0i32, 0i32);
        let inner = unsafe {
            htslib::bam_plp_auto(self.itr, &mut tid, &mut pos, &mut depth)
        };

        match inner.is_null() {
            true if depth == -1 => Some(Err(PileupError::Some)),
            true              => None,
            false             => Some(Ok(
                    Pileup {
                        inner: inner,
                        depth: depth as u32,
                        tid: tid as u32,
                        pos: pos as u32,
                    }
            ))
        }
    }
}


impl Drop for Pileups {
    fn drop(&mut self) {
        unsafe {
            htslib::bam_plp_reset(self.itr);
            htslib::bam_plp_destroy(self.itr);
        }
    }
}


quick_error! {
    #[derive(Debug)]
    pub enum PileupError {
        Some {
            description("error generating pileup")
        }
    }
}
