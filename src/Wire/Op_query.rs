use std::{io::{Cursor, BufRead}, ffi::CString};

use bson::{Document, doc, ser};
use byteorder::{LittleEndian, ReadBytesExt};

use super::{MsgHeader, Replyable, HEADER_SIZE, OpCode, OP_REPLY, Serializable};

use crate::handler::Response;

#[allow(non_camel_case_types)]
#[derive(Debug, Clone)]
pub struct OP_QUERY {
    pub header: MsgHeader,
    pub flags: u32,
    pub collection: String,
    pub number_to_skip: u32,
    pub number_to_return: u32,
    pub query: Document,
    pub return_fields: Option<Document>,
}







impl OP_QUERY {
    pub fn parse(header: MsgHeader, cursor: &mut Cursor<&[u8]>) -> OP_QUERY {
        let flags = cursor.read_u32::<LittleEndian>().unwrap();

        let mut buffer: Vec<u8> = vec![];
        cursor.read_until(0, &mut buffer).unwrap();
        let collection = unsafe { CString::from_vec_unchecked(buffer) }.to_string_lossy().to_string();
        let number_to_skip = cursor.read_u32::<LittleEndian>().unwrap();
        let number_to_return = cursor.read_u32::<LittleEndian>().unwrap();
        let mut new_cursor = cursor.clone();
        new_cursor.set_position(cursor.position());
        let len = new_cursor.get_ref().len();
        if (cursor.position() as usize) < len - 1 {
            return OP_QUERY {
                header: header,
                flags: flags,
                collection: collection,
                number_to_skip: number_to_skip,
                number_to_return: number_to_return,
                query: doc!{},
                return_fields: None,
            };
        }

        let query = Document::from_reader(cursor).unwrap();
        let bson_vec = ser::to_vec(&query).unwrap();
        let query_size: u64 = bson_vec.len().try_into().unwrap();
        new_cursor.set_position(new_cursor.position() + query_size);
        return OP_QUERY {
            header: header,
            flags: flags,
            collection: collection,
            number_to_skip: number_to_skip,
            number_to_return: number_to_return,
            query: query,
            return_fields: match Document::from_reader(new_cursor) {
                Ok(doc) => Some(doc),
                Err(_) => None,
            }
        };
    }

}




impl Replyable for OP_QUERY {
    fn reply(&self, res: Response) -> Result<Vec<u8>, super::UnknownMessageKindError>
    {
        let bson_vector = ser::to_vec(&res.get_doc()).unwrap();
        let bson_data = &bson_vector;
        let message_length = HEADER_SIZE + 20 + bson_data.len() as u32;

        if let OpCode::OpQuery(op_query) = res.get_op_code().to_owned() {
            let header = op_query.header.get_response_with_op_code(res.get_id(), message_length, OP_REPLY);
            let cursor_id = 0;
            let starting_from = 0;
            let number_returned = 1;
            let docs = vec![res.get_doc().to_owned()];
            return Ok(OP_REPLY::new(header,self.flags,cursor_id,starting_from,number_returned,docs).to_vec());
        }
        return Err(super::UnknownMessageKindError);
    }
}
