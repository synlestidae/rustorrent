use bencode::{Bencode, BDict, BString};
use std::{error, fmt};

#[derive(Default)]
pub struct MetaInfo {
    pub announce: String,
    pub announce_list: Vec<String>,
    pub comment: Option<String>,
    pub created_by: Option<String>,
    pub creation_date: Option<u32>,
    pub info: FileInfo,
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

#[derive(Debug)]
pub enum MetaInfoErrorKind {
    MissingField(String),
    FieldIsWrongType(String),
    InvalidDataFieldValue(String),
}
impl MetaInfo {
    pub fn from(dict: &BDict) -> Result<MetaInfo, MetaInfoError> {
        let mut info: MetaInfo = Default::default();
        info.announce = try!(MetaInfo::get_string_or_error(dict, "announce"));
        info.announce_list = MetaInfo::get_vecstring(dict, "announce_list");
        info.comment = MetaInfo::get_optionstring(dict, "comment");
        info.created_by = MetaInfo::get_optionstring(dict, "created_by");
        info.creation_date = MetaInfo::get_u32_or_error(dict, "creation date").ok();
        info.info = try!(MetaInfo::get_info(dict));
        Ok(info)
    }

    fn get_string_or_error(dict: &BDict, field: &str) -> Result<String, MetaInfoError> {
        match dict.get(field) {
            Some(&Bencode::BString(ref bstring)) => {
                let string = bstring.to_string();
                if string.is_ok() {
                    Ok(string.ok().unwrap())
                } else {
                    Err(MetaInfoError::field_type(field))
                }
            }
            Some(_) => Err(MetaInfoError::field_type(field)),
            None => Err(MetaInfoError::missing_field(field)),
        }
    }

    fn get_bstring_or_error(dict: &BDict, field: &str) -> Result<BString, MetaInfoError> {
        match dict.get(field) {
            Some(&Bencode::BString(ref bstring)) => Ok(bstring.clone()),
            Some(_) => Err(MetaInfoError::field_type(field)),
            None => Err(MetaInfoError::missing_field(field)),
        }
    }

    fn get_vecstring(dict: &BDict, field: &str) -> Vec<String> {
        let mut list = Vec::new();
        match dict.get(field) {
            Some(&Bencode::BList(ref blist)) => {
                for item in blist.list().iter() {
                    match item {
                        &Bencode::BString(ref bstring) => {
                            let s = bstring.to_string();
                            if s.is_ok() {
                                list.push(s.ok().unwrap());
                            }
                        }
                        _ => return vec![],
                    }
                }
            }
            _ => (),
        }
        list
    }

    fn get_optionstring(dict: &BDict, field: &str) -> Option<String> {
        match dict.get(field) {
            Some(&Bencode::BString(ref bstring)) => bstring.to_string().ok(),
            _ => None,
        }
    }

    fn get_u32_or_error(dict: &BDict, field: &str) -> Result<u32, MetaInfoError> {
        match dict.get(field) {
            Some(&Bencode::BInt(ref bint)) => Ok(bint.to_i64() as u32),
            Some(_) => Err(MetaInfoError::field_type(field)),
            None => Err(MetaInfoError::missing_field(field)),
        }

    }

    fn get_u64_or_error(dict: &BDict, field: &str) -> Result<u64, MetaInfoError> {
        match dict.get(field) {
            Some(&Bencode::BInt(ref bint)) => Ok(bint.to_i64() as u64),
            Some(_) => Err(MetaInfoError::field_type(field)),
            None => Err(MetaInfoError::missing_field(field)),
        }

    }


    fn get_info(dict: &BDict) -> Result<FileInfo, MetaInfoError> {
        let mut info: FileInfo = Default::default();
        let bdict: &BDict = match try!(dict.get("info")
            .ok_or(MetaInfoError::missing_field("info"))) {
            &Bencode::BDict(ref dict) => dict,
            _ => return Err(MetaInfoError::field_type("info")),
        };
        info.piece_length = try!(MetaInfo::get_u64_or_error(bdict, "piece length"));
        info.private = MetaInfo::get_u32_or_error(bdict, "private").ok();
        let pieces = try!(MetaInfo::get_bstring_or_error(bdict, "pieces")).to_bytes();
        let pieces_vec = (0..pieces.len() / 20)
            .map(|p| (&pieces[p..(p + 20)]).iter().map(|&b| b).collect())
            .collect::<Vec<SHA1Hash20b>>();

        info.pieces = pieces_vec;
        info.name = try!(MetaInfo::get_string_or_error(bdict, "name"));

        if let (Some(&Bencode::BString(ref md5sum)), Some(&Bencode::BInt(ref length))) =
               (bdict.get("md5sum"), bdict.get("length")) {
            info.mode_info = ModeInfo::Single(SingleFileInfo {
                md5_sum: md5sum.to_bytes(),
                length: length.to_i64() as u64,
            });
            return Ok(info);
        };

        if let Some(&Bencode::BList(ref flist)) = bdict.get("files") {
            let mut files = Vec::new();
            for fdict in flist.list() {
                match fdict {
                    &Bencode::BDict(ref bdict) => {
                        let length = try!(MetaInfo::get_u64_or_error(bdict, "length"));
                        let md5_sum = MetaInfo::get_bstring_or_error(bdict, "md5sum").ok().map(|b| b.to_bytes());
                        let path = MetaInfo::get_vecstring(bdict, "path");
                        files.push((length, md5_sum, path));
                    }
                    _ => return Err(MetaInfoError::field_type("files")),
                }
            }
            info.mode_info = ModeInfo::Multi(MultiFileInfo { files: files });
        }

        Ok(info)
    }
}

#[derive(Default)]
pub struct FileInfo {
    piece_length: u64,
    pieces: Vec<SHA1Hash20b>,
    private: Option<u32>,
    name: String,
    mode_info: ModeInfo,
}

pub enum ModeInfo {
    Single(SingleFileInfo),
    Multi(MultiFileInfo),
}

impl Default for ModeInfo {
    fn default() -> ModeInfo {
        ModeInfo::Single(Default::default())
    }
}

#[derive(Debug, Default)]
pub struct SingleFileInfo {
    pub length: u64,
    pub md5_sum: MD5Sum,
}

#[derive(Debug, Default)]
pub struct MultiFileInfo {
    files: Vec<(u64, Option<MD5Sum>, FPath)>,
}

pub type MD5Sum = Vec<u8>;
pub type SHA1Hash20b = Vec<u8>;
pub type FPath = Vec<String>;
