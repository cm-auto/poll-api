use serde::{ser::SerializeStruct, Deserialize, Serialize, Serializer};
use sqlx::types::ipnetwork::IpNetwork;

#[derive(Debug, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "poll_type", rename_all = "snake_case")]
#[serde(rename_all = "camelCase")]
pub enum PollType {
    Single,
    Multiple,
}

#[derive(Debug, sqlx::FromRow, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Poll {
    pub id: i64,
    pub title: String,
    pub poll_type: PollType,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub timeout_at: chrono::DateTime<chrono::Utc>,
    pub delete_at: chrono::DateTime<chrono::Utc>,
}

// basically three models
// a poll
// options for the poll
// votes

// do we need a creator?
// poll:
//      name,
//      type of vote (multiple or one vote),
//      timestamp of creation,
//      how long voting is allowed
//      and when to delete (let's say it can't be bigger than a week and creator can make it shorter)
//      reference to creator? no let's make it simple and just use the ip address

// options:
//      name,
//      reference to poll

#[derive(Debug, sqlx::FromRow, Serialize, Deserialize)]
pub struct PollOption {
    pub id: i64,
    pub name: String,
    pub poll_id: i64,
}

#[derive(Debug, sqlx::FromRow, Serialize, Deserialize)]
pub struct PollVote {
    pub id: i64,
    pub option_id: i64,
    pub ip_address: IpNetwork,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug)]
pub(crate) struct Message<'a>(pub &'a str);

impl<'a> Serialize for Message<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut ts = serializer.serialize_struct("message", 1)?;
        ts.serialize_field("message", self.0)?;
        ts.end()
    }
}
