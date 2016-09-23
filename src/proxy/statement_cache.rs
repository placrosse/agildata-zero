use std::sync::Mutex;
use std::rc::Rc;
use std::collections::HashMap;
use encrypt::{NativeType, EncryptionType};

#[derive(Debug, PartialEq)]
pub enum ValueType {
    BOUND_PARAM(u32),
    LITERAL(u32),
    COLUMN
}

#[derive(Debug, PartialEq)]
pub struct EncryptionPlan {
    data_type: NativeType,
    encryption: EncryptionType,
    value_type: ValueType
}

#[derive(Debug, PartialEq)]
pub struct PPlan {
    literals: Vec<EncryptionPlan>,
    params: Vec<EncryptionPlan>,
    result: Vec<EncryptionPlan>
}

pub enum PhysicalPlan {
    Plan(PPlan),
    Error{message: String, code: String}
}

struct StatementCache {
    cache: Mutex<HashMap<u64, Rc<PhysicalPlan>>>
}

impl StatementCache {
    pub fn new() -> Self {
        StatementCache {
            cache: Mutex::new(HashMap::new())
        }
    }

    pub fn get(&self, key: &u64) -> Option<Rc<PhysicalPlan>> {
        let data = self.cache.lock().unwrap();

        match data.get(key) {
            Some(rc) => Some(rc.clone()),
            None => None
        }
    }

    pub fn put(&self, key: u64, ep: PhysicalPlan) {
        let mut data = self.cache.lock().unwrap();

        data.insert(key, Rc::new(ep));
    }
}