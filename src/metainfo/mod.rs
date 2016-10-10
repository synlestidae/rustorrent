use bencode::{BDict, BString, BInt};
use std::{error, fmt};
use convert::TryFrom;

#[derive(Default, Clone)]
pub struct MetaInfo {
    pub announce: String,
    pub announce_list: Vec<Vec<String>>,
    pub comment: Option<String>,
    pub created_by: Option<String>,
    pub creation_date: Option<u32>,
    pub info: FileInfo,
    pub original: Option<BDict>
}

#[derive(Debug)]
pub struct MetaInfoError {
    kind: MetaInfoErrorKind,
}

impl fmt::Display for MetaInfoError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error while parsing metainfo: {}", self._description())
    }
}

impl MetaInfoError {
    pub fn missing_field(field: &str) -> MetaInfoError {
        MetaInfoError { kind: MetaInfoErrorKind::MissingField(field.to_string()) }
    }

    pub fn field_type(field: &str) -> MetaInfoError {
        MetaInfoError { kind: MetaInfoErrorKind::FieldIsWrongType(field.to_string()) }
    }

    pub fn invalid_data(field: &str) -> MetaInfoError {
        MetaInfoError { kind: MetaInfoErrorKind::InvalidDataFieldValue(field.to_string()) }
    }

    fn _description(&self) -> &str {
        match self.kind {
            MetaInfoErrorKind::MissingField(_) => "required field on file is missing",
            MetaInfoErrorKind::FieldIsWrongType(_) => "required field has wrong bencoding type",
            MetaInfoErrorKind::InvalidDataFieldValue(_) => {
                "required field has correct type but invalid data"
            }
        }
    }
}

impl error::Error for MetaInfoError {
    fn description(&self) -> &str {
        self._description()
    }
}

#[derive(Debug, Clone)]
pub enum MetaInfoErrorKind {
    MissingField(String),
    FieldIsWrongType(String),
    InvalidDataFieldValue(String),
}

impl TryFrom<BDict> for MetaInfo {
    type Err = MetaInfoError;

    fn try_from(bdict: BDict) -> Result<MetaInfo, MetaInfoError> {
        let dict = &bdict;
        let mut info: MetaInfo = Default::default();

        let announce: Option<String> = dict.get_copy("announce");
        let announce_list: Option<Vec<Vec<String>>> = dict.get_copy("announce_list");
        let comment: Option<String> = dict.get_copy("comment");
        let created_by: Option<String> = dict.get_copy("created_by");
        let creation_date: Option<BInt> = dict.get_copy("creation date");

        info.announce = try!(announce.ok_or(MetaInfoError::missing_field("annouce")));
        info.announce_list = announce_list.unwrap_or(Vec::new());
        info.comment = comment; //try!(comment.ok_or(MetaInfoError::missing_field("comment")));
        info.created_by = created_by;
        info.creation_date = creation_date.map(|bdict| bdict.to_i64() as u32);
        info.info = try!(MetaInfo::get_info(dict));

        Ok(info)
    }
}

impl Into<BDict> for MetaInfo {
    fn into(self) -> BDict {
        unimplemented!();
    }
}


impl MetaInfo {
    fn get_info(dict: &BDict) -> Result<FileInfo, MetaInfoError> {
        let mut info: FileInfo = Default::default();
        let bdict: BDict = try!(dict.get_copy("info").ok_or(MetaInfoError::missing_field("info")));
        info.piece_length = try!(bdict.get_copy("piece length")
            .map(|pl: BInt| pl.to_i64() as u64)
            .ok_or(MetaInfoError::missing_field("piece length")));
        info.private = bdict.get_copy("private").map(|p: BInt| p.to_i64() as u32);
        let pieces_bstr: BString = try!(bdict.get_copy("pieces")
            .ok_or(MetaInfoError::missing_field("pieces")));
        let pieces = pieces_bstr.to_bytes();
        let pieces_vec = (0..pieces.len() / 20)
            .map(|p| (&pieces[p..(p + 20)]).iter().map(|&b| b).collect())
            .collect::<Vec<SHA1Hash20b>>();

        info.pieces = pieces_vec;
        info.name = bdict.get_copy("name");

        let single_file_fields: (Option<BString>, Option<BInt>) = (bdict.get_copy("md5sum"),
                                                                   bdict.get_copy("length"));
        if let (md5sum, Some(length)) = single_file_fields {
            info.mode_info = ModeInfo::Single(SingleFileInfo {
                md5_sum: md5sum.map(|m| m.to_bytes()),
                length: length.to_i64() as u64,
            });
            info.original = Some(bdict);
            return Ok(info);
        }

        let bdict_files = bdict.get_copy("files");
        let bdict_list: Vec<BDict> = try!(bdict_files.ok_or(MetaInfoError::missing_field("files")));
        let mut files = Vec::new();
        for fdict in bdict_list.into_iter() {
            let length: BInt = try!(fdict.get_copy("length")
                .ok_or(MetaInfoError::missing_field("length")));
            let md5_sum = fdict.get_copy("md5sum").map(|m: BString| m.to_bytes());
            let path: Vec<String> = try!(fdict.get_copy("path")
                .ok_or(MetaInfoError::missing_field("path")));
            files.push((length.to_i64() as u64, md5_sum, path));
        }

        info.mode_info = ModeInfo::Multi(MultiFileInfo { files: files });
        info.original = Some(bdict);
        Ok(info)
    }
}

#[derive(Default, Clone)]
pub struct FileInfo {
    pub piece_length: u64,
    pub pieces: Vec<SHA1Hash20b>,
    pub private: Option<u32>,
    pub name: Option<String>,
    pub mode_info: ModeInfo,
    pub original: Option<BDict>
}

#[derive(Clone)]
pub enum ModeInfo {
    Single(SingleFileInfo),
    Multi(MultiFileInfo),
}

impl Default for ModeInfo {
    fn default() -> ModeInfo {
        ModeInfo::Single(Default::default())
    }
}

#[derive(Debug, Default, Clone)]
pub struct SingleFileInfo {
    pub length: u64,
    pub md5_sum: Option<MD5Sum>,
}

#[derive(Debug, Default, Clone)]
pub struct MultiFileInfo {
    pub files: Vec<(u64, Option<MD5Sum>, FPath)>,
}

pub type MD5Sum = Vec<u8>;
pub type SHA1Hash20b = Vec<u8>;
pub type FPath = Vec<String>;
