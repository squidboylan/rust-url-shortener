use super::schema::links;

#[derive(Queryable, Serialize, Deserialize)]
pub struct Link {
    pub id: String,
    pub dest_url: String,
    pub count: i32,
}

#[derive(Insertable, Serialize, Deserialize)]
#[table_name="links"]
pub struct LinkCreate {
    pub dest_url: String,
}
