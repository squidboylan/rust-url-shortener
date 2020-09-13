use super::schema::links;

#[derive(Clone, Default, Debug, Queryable, Serialize, Deserialize)]
pub struct Link {
    pub id: String,
    pub dest_url: String,
    pub count: i32,
}

#[derive(Clone, Insertable, Serialize, Deserialize)]
#[table_name="links"]
pub struct LinkCreate {
    pub dest_url: String,
}
