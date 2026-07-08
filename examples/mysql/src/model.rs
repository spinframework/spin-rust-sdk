use spin_sdk::mysql;

// Such logic, very business

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct Pet {
    id: i32,
    name: String,
    prey: Option<String>,
    is_finicky: bool,
}

pub(crate) fn as_pet(row: &mysql::Row) -> Option<Pet> {
    let id = row.get("id")?;
    let name = row.get("name")?;
    let prey = row.get("prey")?;
    let is_finicky = row.get("is_finicky")?;

    Some(Pet {
        id,
        name,
        prey,
        is_finicky,
    })
}
