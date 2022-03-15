use rdkafka::{
    admin::{AdminClient, AdminOptions, NewTopic, TopicReplication},
    client::DefaultClientContext,
    ClientConfig,
};
use crate::KAFKA_TOPIC;


pub async fn create_stresstest_topic() {
    let client: AdminClient<DefaultClientContext> = ClientConfig::new()
        .set("bootstrap.servers", "localhost")
        .create()
        .expect("AdminClient creation error");

    let topics = [KAFKA_TOPIC];
    let opts = AdminOptions::new();
    client
        .delete_topics(&topics, &opts)
        .await
        .expect("Error deleting topic");

    let new_topics = [NewTopic::new(
        KAFKA_TOPIC,
        1,
        TopicReplication::Fixed(1),
    )];
    client
        .create_topics(&new_topics, &opts)
        .await
        .expect("Error creating topic");
}
