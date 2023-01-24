use actix_web::test;
use meili_snap::{json_string, snapshot};
use serde_json::{json, Value};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::common::encoder::Encoder;
use crate::common::{GetAllDocumentsOptions, Server};

/// This is the basic usage of our API and every other tests uses the content-type application/json
#[actix_rt::test]
async fn add_documents_test_json_content_types() {
    let document = json!([
        {
            "id": 1,
            "content": "Bouvier Bernois",
        }
    ]);

    // this is a what is expected and should work
    let server = Server::new().await;
    let app = server.init_web_app().await;

    // post
    let req = test::TestRequest::post()
        .uri("/indexes/dog/documents")
        .set_payload(document.to_string())
        .insert_header(("content-type", "application/json"))
        .to_request();
    let res = test::call_service(&app, req).await;
    let status_code = res.status();
    let body = test::read_body(res).await;
    let response: Value = serde_json::from_slice(&body).unwrap_or_default();
    snapshot!(status_code, @"202 Accepted");
    snapshot!(json_string!(response, { ".duration" => "[duration]", ".enqueuedAt" => "[date]", ".startedAt" => "[date]", ".finishedAt" => "[date]" }),
        @r###"
    {
      "taskUid": 0,
      "indexUid": "dog",
      "status": "enqueued",
      "type": "documentAdditionOrUpdate",
      "enqueuedAt": "[date]"
    }
    "###);

    // put
    let req = test::TestRequest::put()
        .uri("/indexes/dog/documents")
        .set_payload(document.to_string())
        .insert_header(("content-type", "application/json"))
        .to_request();
    let res = test::call_service(&app, req).await;
    let status_code = res.status();
    let body = test::read_body(res).await;
    let response: Value = serde_json::from_slice(&body).unwrap_or_default();
    snapshot!(status_code, @"202 Accepted");
    snapshot!(json_string!(response, { ".duration" => "[duration]", ".enqueuedAt" => "[date]", ".startedAt" => "[date]", ".finishedAt" => "[date]" }),
        @r###"
    {
      "taskUid": 1,
      "indexUid": "dog",
      "status": "enqueued",
      "type": "documentAdditionOrUpdate",
      "enqueuedAt": "[date]"
    }
    "###);
}

/// Here we try to send a single document instead of an array with a single document inside.
#[actix_rt::test]
async fn add_single_document_test_json_content_types() {
    let document = json!({
        "id": 1,
        "content": "Bouvier Bernois",
    });

    // this is a what is expected and should work
    let server = Server::new().await;
    let app = server.init_web_app().await;

    // post
    let req = test::TestRequest::post()
        .uri("/indexes/dog/documents")
        .set_payload(document.to_string())
        .insert_header(("content-type", "application/json"))
        .to_request();
    let res = test::call_service(&app, req).await;
    let status_code = res.status();
    let body = test::read_body(res).await;
    let response: Value = serde_json::from_slice(&body).unwrap_or_default();
    snapshot!(status_code, @"202 Accepted");
    snapshot!(json_string!(response, { ".duration" => "[duration]", ".enqueuedAt" => "[date]", ".startedAt" => "[date]", ".finishedAt" => "[date]" }),
        @r###"
    {
      "taskUid": 0,
      "indexUid": "dog",
      "status": "enqueued",
      "type": "documentAdditionOrUpdate",
      "enqueuedAt": "[date]"
    }
    "###);

    // put
    let req = test::TestRequest::put()
        .uri("/indexes/dog/documents")
        .set_payload(document.to_string())
        .insert_header(("content-type", "application/json"))
        .to_request();
    let res = test::call_service(&app, req).await;
    let status_code = res.status();
    let body = test::read_body(res).await;
    let response: Value = serde_json::from_slice(&body).unwrap_or_default();
    snapshot!(status_code, @"202 Accepted");
    snapshot!(json_string!(response, { ".duration" => "[duration]", ".enqueuedAt" => "[date]", ".startedAt" => "[date]", ".finishedAt" => "[date]" }),
        @r###"
    {
      "taskUid": 1,
      "indexUid": "dog",
      "status": "enqueued",
      "type": "documentAdditionOrUpdate",
      "enqueuedAt": "[date]"
    }
    "###);
}

/// Here we try sending encoded (compressed) document request
#[actix_rt::test]
async fn add_single_document_gzip_encoded() {
    let document = json!({
        "id": 1,
        "content": "Bouvier Bernois",
    });

    // this is a what is expected and should work
    let server = Server::new().await;
    let app = server.init_web_app().await;
    // post
    let document = serde_json::to_string(&document).unwrap();
    let encoder = Encoder::Gzip;
    let req = test::TestRequest::post()
        .uri("/indexes/dog/documents")
        .set_payload(encoder.encode(document.clone()))
        .insert_header(("content-type", "application/json"))
        .insert_header(encoder.header().unwrap())
        .to_request();
    let res = test::call_service(&app, req).await;
    let status_code = res.status();
    let body = test::read_body(res).await;
    let response: Value = serde_json::from_slice(&body).unwrap_or_default();
    snapshot!(status_code, @"202 Accepted");
    snapshot!(json_string!(response, { ".duration" => "[duration]", ".enqueuedAt" => "[date]", ".startedAt" => "[date]", ".finishedAt" => "[date]" }),
        @r###"
    {
      "taskUid": 0,
      "indexUid": "dog",
      "status": "enqueued",
      "type": "documentAdditionOrUpdate",
      "enqueuedAt": "[date]"
    }
    "###);

    // put
    let req = test::TestRequest::put()
        .uri("/indexes/dog/documents")
        .set_payload(encoder.encode(document))
        .insert_header(("content-type", "application/json"))
        .insert_header(encoder.header().unwrap())
        .to_request();
    let res = test::call_service(&app, req).await;
    let status_code = res.status();
    let body = test::read_body(res).await;
    let response: Value = serde_json::from_slice(&body).unwrap_or_default();
    snapshot!(status_code, @"202 Accepted");
    snapshot!(json_string!(response, { ".duration" => "[duration]", ".enqueuedAt" => "[date]", ".startedAt" => "[date]", ".finishedAt" => "[date]" }),
        @r###"
    {
      "taskUid": 1,
      "indexUid": "dog",
      "status": "enqueued",
      "type": "documentAdditionOrUpdate",
      "enqueuedAt": "[date]"
    }
    "###);
}

/// Here we try document request with every encoding
#[actix_rt::test]
async fn add_single_document_with_every_encoding() {
    let document = json!({
        "id": 1,
        "content": "Bouvier Bernois",
    });

    // this is a what is expected and should work
    let server = Server::new().await;
    let app = server.init_web_app().await;
    // post
    let document = serde_json::to_string(&document).unwrap();

    for (task_uid, encoder) in Encoder::iterator().enumerate() {
        let mut req = test::TestRequest::post()
            .uri("/indexes/dog/documents")
            .set_payload(encoder.encode(document.clone()))
            .insert_header(("content-type", "application/json"));
        req = match encoder.header() {
            Some(header) => req.insert_header(header),
            None => req,
        };
        let req = req.to_request();
        let res = test::call_service(&app, req).await;
        let status_code = res.status();
        let body = test::read_body(res).await;
        let response: Value = serde_json::from_slice(&body).unwrap_or_default();
        assert_eq!(status_code, 202);
        assert_eq!(response["taskUid"], task_uid);
    }
}

/// any other content-type is must be refused
#[actix_rt::test]
async fn error_add_documents_test_bad_content_types() {
    let document = json!([
        {
            "id": 1,
            "content": "Leonberg",
        }
    ]);

    let server = Server::new().await;
    let app = server.init_web_app().await;

    // post
    let req = test::TestRequest::post()
        .uri("/indexes/dog/documents")
        .set_payload(document.to_string())
        .insert_header(("content-type", "text/plain"))
        .to_request();
    let res = test::call_service(&app, req).await;
    let status_code = res.status();
    let body = test::read_body(res).await;
    let response: Value = serde_json::from_slice(&body).unwrap_or_default();
    snapshot!(status_code, @"415 Unsupported Media Type");
    snapshot!(json_string!(response),
        @r###"
    {
      "message": "The Content-Type `text/plain` is invalid. Accepted values for the Content-Type header are: `application/json`, `application/x-ndjson`, `text/csv`",
      "code": "invalid_content_type",
      "type": "invalid_request",
      "link": "https://docs.meilisearch.com/errors#invalid_content_type"
    }
    "###);

    // put
    let req = test::TestRequest::put()
        .uri("/indexes/dog/documents")
        .set_payload(document.to_string())
        .insert_header(("content-type", "text/plain"))
        .to_request();
    let res = test::call_service(&app, req).await;
    let status_code = res.status();
    let body = test::read_body(res).await;
    let response: Value = serde_json::from_slice(&body).unwrap_or_default();
    snapshot!(status_code, @"415 Unsupported Media Type");
    snapshot!(json_string!(response),
        @r###"
    {
      "message": "The Content-Type `text/plain` is invalid. Accepted values for the Content-Type header are: `application/json`, `application/x-ndjson`, `text/csv`",
      "code": "invalid_content_type",
      "type": "invalid_request",
      "link": "https://docs.meilisearch.com/errors#invalid_content_type"
    }
    "###);
}

/// missing content-type must be refused
#[actix_rt::test]
async fn error_add_documents_test_no_content_type() {
    let document = json!([
        {
            "id": 1,
            "content": "Leonberg",
        }
    ]);

    let server = Server::new().await;
    let app = server.init_web_app().await;

    // post
    let req = test::TestRequest::post()
        .uri("/indexes/dog/documents")
        .set_payload(document.to_string())
        .to_request();
    let res = test::call_service(&app, req).await;
    let status_code = res.status();
    let body = test::read_body(res).await;
    let response: Value = serde_json::from_slice(&body).unwrap_or_default();
    snapshot!(status_code, @"415 Unsupported Media Type");
    snapshot!(json_string!(response),
        @r###"
    {
      "message": "A Content-Type header is missing. Accepted values for the Content-Type header are: `application/json`, `application/x-ndjson`, `text/csv`",
      "code": "missing_content_type",
      "type": "invalid_request",
      "link": "https://docs.meilisearch.com/errors#missing_content_type"
    }
    "###);

    // put
    let req = test::TestRequest::put()
        .uri("/indexes/dog/documents")
        .set_payload(document.to_string())
        .to_request();
    let res = test::call_service(&app, req).await;
    let status_code = res.status();
    let body = test::read_body(res).await;
    let response: Value = serde_json::from_slice(&body).unwrap_or_default();
    snapshot!(status_code, @"415 Unsupported Media Type");
    snapshot!(json_string!(response),
        @r###"
    {
      "message": "A Content-Type header is missing. Accepted values for the Content-Type header are: `application/json`, `application/x-ndjson`, `text/csv`",
      "code": "missing_content_type",
      "type": "invalid_request",
      "link": "https://docs.meilisearch.com/errors#missing_content_type"
    }
    "###);
}

#[actix_rt::test]
async fn error_add_malformed_csv_documents() {
    let document = "id, content\n1234, hello, world\n12, hello world";

    let server = Server::new().await;
    let app = server.init_web_app().await;

    // post
    let req = test::TestRequest::post()
        .uri("/indexes/dog/documents")
        .set_payload(document.to_string())
        .insert_header(("content-type", "text/csv"))
        .to_request();
    let res = test::call_service(&app, req).await;
    let status_code = res.status();
    let body = test::read_body(res).await;
    let response: Value = serde_json::from_slice(&body).unwrap_or_default();
    snapshot!(status_code, @"400 Bad Request");
    snapshot!(json_string!(response),
        @r###"
    {
      "message": "The `csv` payload provided is malformed: `CSV error: record 1 (line: 2, byte: 12): found record with 3 fields, but the previous record has 2 fields`.",
      "code": "malformed_payload",
      "type": "invalid_request",
      "link": "https://docs.meilisearch.com/errors#malformed_payload"
    }
    "###);

    // put
    let req = test::TestRequest::put()
        .uri("/indexes/dog/documents")
        .set_payload(document.to_string())
        .insert_header(("content-type", "text/csv"))
        .to_request();
    let res = test::call_service(&app, req).await;
    let status_code = res.status();
    let body = test::read_body(res).await;
    let response: Value = serde_json::from_slice(&body).unwrap_or_default();
    snapshot!(status_code, @"400 Bad Request");
    snapshot!(json_string!(response),
        @r###"
    {
      "message": "The `csv` payload provided is malformed: `CSV error: record 1 (line: 2, byte: 12): found record with 3 fields, but the previous record has 2 fields`.",
      "code": "malformed_payload",
      "type": "invalid_request",
      "link": "https://docs.meilisearch.com/errors#malformed_payload"
    }
    "###);
}

#[actix_rt::test]
async fn error_add_malformed_json_documents() {
    let document = r#"[{"id": 1}, {id: 2}]"#;

    let server = Server::new().await;
    let app = server.init_web_app().await;

    // post
    let req = test::TestRequest::post()
        .uri("/indexes/dog/documents")
        .set_payload(document.to_string())
        .insert_header(("content-type", "application/json"))
        .to_request();
    let res = test::call_service(&app, req).await;
    let status_code = res.status();
    let body = test::read_body(res).await;
    let response: Value = serde_json::from_slice(&body).unwrap_or_default();
    snapshot!(status_code, @"400 Bad Request");
    snapshot!(json_string!(response),
        @r###"
    {
      "message": "The `json` payload provided is malformed. `Couldn't serialize document value: key must be a string at line 1 column 14`.",
      "code": "malformed_payload",
      "type": "invalid_request",
      "link": "https://docs.meilisearch.com/errors#malformed_payload"
    }
    "###);

    // put
    let req = test::TestRequest::put()
        .uri("/indexes/dog/documents")
        .set_payload(document.to_string())
        .insert_header(("content-type", "application/json"))
        .to_request();
    let res = test::call_service(&app, req).await;
    let status_code = res.status();
    let body = test::read_body(res).await;
    let response: Value = serde_json::from_slice(&body).unwrap_or_default();
    snapshot!(status_code, @"400 Bad Request");
    snapshot!(json_string!(response),
        @r###"
    {
      "message": "The `json` payload provided is malformed. `Couldn't serialize document value: key must be a string at line 1 column 14`.",
      "code": "malformed_payload",
      "type": "invalid_request",
      "link": "https://docs.meilisearch.com/errors#malformed_payload"
    }
    "###);

    // truncate

    // length = 100
    let long = "0123456789".repeat(10);

    let document = format!("\"{}\"", long);
    let req = test::TestRequest::put()
        .uri("/indexes/dog/documents")
        .set_payload(document)
        .insert_header(("content-type", "application/json"))
        .to_request();
    let res = test::call_service(&app, req).await;
    let body = test::read_body(res).await;
    let response: Value = serde_json::from_slice(&body).unwrap_or_default();
    snapshot!(status_code, @"400 Bad Request");
    snapshot!(json_string!(response),
        @r###"
    {
      "message": "The `json` payload provided is malformed. `Couldn't serialize document value: data are neither an object nor a list of objects`.",
      "code": "malformed_payload",
      "type": "invalid_request",
      "link": "https://docs.meilisearch.com/errors#malformed_payload"
    }
    "###);

    // add one more char to the long string to test if the truncating works.
    let document = format!("\"{}m\"", long);
    let req = test::TestRequest::put()
        .uri("/indexes/dog/documents")
        .set_payload(document)
        .insert_header(("content-type", "application/json"))
        .to_request();
    let res = test::call_service(&app, req).await;
    let body = test::read_body(res).await;
    let response: Value = serde_json::from_slice(&body).unwrap_or_default();
    snapshot!(status_code, @"400 Bad Request");
    snapshot!(json_string!(response),
        @r###"
    {
      "message": "The `json` payload provided is malformed. `Couldn't serialize document value: data are neither an object nor a list of objects`.",
      "code": "malformed_payload",
      "type": "invalid_request",
      "link": "https://docs.meilisearch.com/errors#malformed_payload"
    }
    "###);
}

#[actix_rt::test]
async fn error_add_malformed_ndjson_documents() {
    let document = "{\"id\": 1}\n{id: 2}";

    let server = Server::new().await;
    let app = server.init_web_app().await;

    // post
    let req = test::TestRequest::post()
        .uri("/indexes/dog/documents")
        .set_payload(document.to_string())
        .insert_header(("content-type", "application/x-ndjson"))
        .to_request();
    let res = test::call_service(&app, req).await;
    let status_code = res.status();
    let body = test::read_body(res).await;
    let response: Value = serde_json::from_slice(&body).unwrap_or_default();
    snapshot!(status_code, @"400 Bad Request");
    snapshot!(json_string!(response),
        @r###"
    {
      "message": "The `ndjson` payload provided is malformed. `Couldn't serialize document value: key must be a string at line 2 column 2`.",
      "code": "malformed_payload",
      "type": "invalid_request",
      "link": "https://docs.meilisearch.com/errors#malformed_payload"
    }
    "###);

    // put
    let req = test::TestRequest::put()
        .uri("/indexes/dog/documents")
        .set_payload(document.to_string())
        .insert_header(("content-type", "application/x-ndjson"))
        .to_request();
    let res = test::call_service(&app, req).await;
    let status_code = res.status();
    let body = test::read_body(res).await;
    let response: Value = serde_json::from_slice(&body).unwrap_or_default();
    snapshot!(status_code, @"400 Bad Request");
    snapshot!(json_string!(response),
        @r###"
    {
      "message": "The `ndjson` payload provided is malformed. `Couldn't serialize document value: key must be a string at line 2 column 2`.",
      "code": "malformed_payload",
      "type": "invalid_request",
      "link": "https://docs.meilisearch.com/errors#malformed_payload"
    }
    "###);
}

#[actix_rt::test]
async fn error_add_missing_payload_csv_documents() {
    let document = "";

    let server = Server::new().await;
    let app = server.init_web_app().await;

    // post
    let req = test::TestRequest::post()
        .uri("/indexes/dog/documents")
        .set_payload(document.to_string())
        .insert_header(("content-type", "text/csv"))
        .to_request();
    let res = test::call_service(&app, req).await;
    let status_code = res.status();
    let body = test::read_body(res).await;
    let response: Value = serde_json::from_slice(&body).unwrap_or_default();
    snapshot!(status_code, @"400 Bad Request");
    snapshot!(json_string!(response),
        @r###"
    {
      "message": "A csv payload is missing.",
      "code": "missing_payload",
      "type": "invalid_request",
      "link": "https://docs.meilisearch.com/errors#missing_payload"
    }
    "###);

    // put
    let req = test::TestRequest::put()
        .uri("/indexes/dog/documents")
        .set_payload(document.to_string())
        .insert_header(("content-type", "text/csv"))
        .to_request();
    let res = test::call_service(&app, req).await;
    let status_code = res.status();
    let body = test::read_body(res).await;
    let response: Value = serde_json::from_slice(&body).unwrap_or_default();
    snapshot!(status_code, @"400 Bad Request");
    snapshot!(json_string!(response),
        @r###"
    {
      "message": "A csv payload is missing.",
      "code": "missing_payload",
      "type": "invalid_request",
      "link": "https://docs.meilisearch.com/errors#missing_payload"
    }
    "###);
}

#[actix_rt::test]
async fn error_add_missing_payload_json_documents() {
    let document = "";

    let server = Server::new().await;
    let app = server.init_web_app().await;

    // post
    let req = test::TestRequest::post()
        .uri("/indexes/dog/documents")
        .set_payload(document.to_string())
        .insert_header(("content-type", "application/json"))
        .to_request();
    let res = test::call_service(&app, req).await;
    let status_code = res.status();
    let body = test::read_body(res).await;
    let response: Value = serde_json::from_slice(&body).unwrap_or_default();
    snapshot!(status_code, @"400 Bad Request");
    snapshot!(json_string!(response),
        @r###"
    {
      "message": "A json payload is missing.",
      "code": "missing_payload",
      "type": "invalid_request",
      "link": "https://docs.meilisearch.com/errors#missing_payload"
    }
    "###);

    // put
    let req = test::TestRequest::put()
        .uri("/indexes/dog/documents")
        .set_payload(document.to_string())
        .insert_header(("content-type", "application/json"))
        .to_request();
    let res = test::call_service(&app, req).await;
    let status_code = res.status();
    let body = test::read_body(res).await;
    let response: Value = serde_json::from_slice(&body).unwrap_or_default();
    snapshot!(status_code, @"400 Bad Request");
    snapshot!(json_string!(response),
        @r###"
    {
      "message": "A json payload is missing.",
      "code": "missing_payload",
      "type": "invalid_request",
      "link": "https://docs.meilisearch.com/errors#missing_payload"
    }
    "###);
}

#[actix_rt::test]
async fn error_add_missing_payload_ndjson_documents() {
    let document = "";

    let server = Server::new().await;
    let app = server.init_web_app().await;

    // post
    let req = test::TestRequest::post()
        .uri("/indexes/dog/documents")
        .set_payload(document.to_string())
        .insert_header(("content-type", "application/x-ndjson"))
        .to_request();
    let res = test::call_service(&app, req).await;
    let status_code = res.status();
    let body = test::read_body(res).await;
    let response: Value = serde_json::from_slice(&body).unwrap_or_default();
    snapshot!(status_code, @"400 Bad Request");
    snapshot!(json_string!(response),
        @r###"
    {
      "message": "A ndjson payload is missing.",
      "code": "missing_payload",
      "type": "invalid_request",
      "link": "https://docs.meilisearch.com/errors#missing_payload"
    }
    "###);

    // put
    let req = test::TestRequest::put()
        .uri("/indexes/dog/documents")
        .set_payload(document.to_string())
        .insert_header(("content-type", "application/x-ndjson"))
        .to_request();
    let res = test::call_service(&app, req).await;
    let status_code = res.status();
    let body = test::read_body(res).await;
    let response: Value = serde_json::from_slice(&body).unwrap_or_default();
    snapshot!(status_code, @"400 Bad Request");
    snapshot!(json_string!(response),
        @r###"
    {
      "message": "A ndjson payload is missing.",
      "code": "missing_payload",
      "type": "invalid_request",
      "link": "https://docs.meilisearch.com/errors#missing_payload"
    }
    "###);
}

#[actix_rt::test]
async fn add_documents_no_index_creation() {
    let server = Server::new().await;
    let index = server.index("test");

    let documents = json!([
        {
            "id": 1,
            "content": "foo",
        }
    ]);

    let (response, code) = index.add_documents(documents, None).await;
    snapshot!(code, @"202 Accepted");
    assert_eq!(response["taskUid"], 0);

    index.wait_task(0).await;

    let (response, code) = index.get_task(0).await;
    snapshot!(code, @"200 OK");
    snapshot!(json_string!(response, { ".duration" => "[duration]", ".enqueuedAt" => "[date]", ".startedAt" => "[date]", ".finishedAt" => "[date]" }),
        @r###"
    {
      "uid": 0,
      "indexUid": "test",
      "status": "succeeded",
      "type": "documentAdditionOrUpdate",
      "canceledBy": null,
      "details": {
        "receivedDocuments": 1,
        "indexedDocuments": 1
      },
      "error": null,
      "duration": "[duration]",
      "enqueuedAt": "[date]",
      "startedAt": "[date]",
      "finishedAt": "[date]"
    }
    "###);

    let processed_at =
        OffsetDateTime::parse(response["finishedAt"].as_str().unwrap(), &Rfc3339).unwrap();
    let enqueued_at =
        OffsetDateTime::parse(response["enqueuedAt"].as_str().unwrap(), &Rfc3339).unwrap();
    assert!(processed_at > enqueued_at);

    // index was created, and primary key was inferred.
    let (response, code) = index.get().await;
    snapshot!(code, @"200 OK");
    assert_eq!(response["primaryKey"], "id");
}

#[actix_rt::test]
async fn error_document_add_create_index_bad_uid() {
    let server = Server::new().await;
    let index = server.index("883  fj!");
    let (response, code) = index.add_documents(json!([{"id": 1}]), None).await;

    snapshot!(code, @"400 Bad Request");
    snapshot!(json_string!(response),
        @r###"
    {
      "message": "`883  fj!` is not a valid index uid. Index uid can be an integer or a string containing only alphanumeric characters, hyphens (-) and underscores (_).",
      "code": "invalid_index_uid",
      "type": "invalid_request",
      "link": "https://docs.meilisearch.com/errors#invalid_index_uid"
    }
    "###);
}

#[actix_rt::test]
async fn document_addition_with_primary_key() {
    let server = Server::new().await;
    let index = server.index("test");

    let documents = json!([
        {
            "primary": 1,
            "content": "foo",
        }
    ]);
    let (response, code) = index.add_documents(documents, Some("primary")).await;
    snapshot!(code, @"202 Accepted");
    snapshot!(json_string!(response, { ".duration" => "[duration]", ".enqueuedAt" => "[date]", ".startedAt" => "[date]", ".finishedAt" => "[date]" }),
        @r###"
    {
      "taskUid": 0,
      "indexUid": "test",
      "status": "enqueued",
      "type": "documentAdditionOrUpdate",
      "enqueuedAt": "[date]"
    }
    "###);

    index.wait_task(0).await;

    let (response, code) = index.get_task(0).await;
    snapshot!(code, @"200 OK");
    snapshot!(json_string!(response, { ".duration" => "[duration]", ".enqueuedAt" => "[date]", ".startedAt" => "[date]", ".finishedAt" => "[date]" }),
        @r###"
    {
      "uid": 0,
      "indexUid": "test",
      "status": "succeeded",
      "type": "documentAdditionOrUpdate",
      "canceledBy": null,
      "details": {
        "receivedDocuments": 1,
        "indexedDocuments": 1
      },
      "error": null,
      "duration": "[duration]",
      "enqueuedAt": "[date]",
      "startedAt": "[date]",
      "finishedAt": "[date]"
    }
    "###);

    let (response, code) = index.get().await;
    snapshot!(code, @"200 OK");
    snapshot!(json_string!(response, { ".createdAt" => "[date]", ".updatedAt" => "[date]" }),
        @r###"
    {
      "uid": "test",
      "createdAt": "[date]",
      "updatedAt": "[date]",
      "primaryKey": "primary"
    }
    "###);
}

#[actix_rt::test]
async fn replace_document() {
    let server = Server::new().await;
    let index = server.index("test");

    let documents = json!([
        {
            "doc_id": 1,
            "content": "foo",
        }
    ]);

    let (response, code) = index.add_documents(documents, None).await;
    snapshot!(code,@"202 Accepted");
    snapshot!(json_string!(response, { ".duration" => "[duration]", ".enqueuedAt" => "[date]", ".startedAt" => "[date]", ".finishedAt" => "[date]" }),
        @r###"
    {
      "taskUid": 0,
      "indexUid": "test",
      "status": "enqueued",
      "type": "documentAdditionOrUpdate",
      "enqueuedAt": "[date]"
    }
    "###);

    index.wait_task(0).await;

    let documents = json!([
        {
            "doc_id": 1,
            "other": "bar",
        }
    ]);

    let (_response, code) = index.add_documents(documents, None).await;
    snapshot!(code,@"202 Accepted");

    index.wait_task(1).await;

    let (response, code) = index.get_task(1).await;
    snapshot!(code, @"200 OK");
    snapshot!(json_string!(response, { ".duration" => "[duration]", ".enqueuedAt" => "[date]", ".startedAt" => "[date]", ".finishedAt" => "[date]" }),
        @r###"
    {
      "uid": 1,
      "indexUid": "test",
      "status": "succeeded",
      "type": "documentAdditionOrUpdate",
      "canceledBy": null,
      "details": {
        "receivedDocuments": 1,
        "indexedDocuments": 1
      },
      "error": null,
      "duration": "[duration]",
      "enqueuedAt": "[date]",
      "startedAt": "[date]",
      "finishedAt": "[date]"
    }
    "###);

    let (response, code) = index.get_document(1, None).await;
    snapshot!(code, @"200 OK");
    snapshot!(json_string!(response),
        @r###"
    {
      "doc_id": 1,
      "other": "bar"
    }
    "###);
}

#[actix_rt::test]
async fn add_no_documents() {
    let server = Server::new().await;
    let index = server.index("test");
    let (_response, code) = index.add_documents(json!([]), None).await;
    snapshot!(code, @"202 Accepted");
}

#[actix_rt::test]
async fn add_larger_dataset() {
    let server = Server::new().await;
    let index = server.index("test");
    let update_id = index.load_test_set().await;
    let (response, code) = index.get_task(update_id).await;
    assert_eq!(code, 200);
    assert_eq!(response["status"], "succeeded");
    assert_eq!(response["type"], "documentAdditionOrUpdate");
    assert_eq!(response["details"]["indexedDocuments"], 77);
    assert_eq!(response["details"]["receivedDocuments"], 77);
    let (response, code) = index
        .get_all_documents(GetAllDocumentsOptions { limit: Some(1000), ..Default::default() })
        .await;
    assert_eq!(code, 200, "failed with `{}`", response);
    assert_eq!(response["results"].as_array().unwrap().len(), 77);

    // x-ndjson add large test
    let server = Server::new().await;
    let index = server.index("test");
    let update_id = index.load_test_set_ndjson().await;
    let (response, code) = index.get_task(update_id).await;
    assert_eq!(code, 200);
    assert_eq!(response["status"], "succeeded");
    assert_eq!(response["type"], "documentAdditionOrUpdate");
    assert_eq!(response["details"]["indexedDocuments"], 77);
    assert_eq!(response["details"]["receivedDocuments"], 77);
    let (response, code) = index
        .get_all_documents(GetAllDocumentsOptions { limit: Some(1000), ..Default::default() })
        .await;
    assert_eq!(code, 200, "failed with `{}`", response);
    assert_eq!(response["results"].as_array().unwrap().len(), 77);
}

#[actix_rt::test]
async fn error_add_documents_bad_document_id() {
    let server = Server::new().await;
    let index = server.index("test");
    index.create(Some("docid")).await;
    let documents = json!([
        {
            "docid": "foo & bar",
            "content": "foobar"
        }
    ]);
    index.add_documents(documents, None).await;
    index.wait_task(1).await;
    let (response, code) = index.get_task(1).await;
    snapshot!(code, @"200 OK");
    snapshot!(json_string!(response, { ".duration" => "[duration]", ".enqueuedAt" => "[date]", ".startedAt" => "[date]", ".finishedAt" => "[date]" }),
        @r###"
    {
      "uid": 1,
      "indexUid": "test",
      "status": "failed",
      "type": "documentAdditionOrUpdate",
      "canceledBy": null,
      "details": {
        "receivedDocuments": 1,
        "indexedDocuments": 1
      },
      "error": {
        "message": "Document identifier `\"foo & bar\"` is invalid. A document identifier can be of type integer or string, only composed of alphanumeric characters (a-z A-Z 0-9), hyphens (-) and underscores (_).",
        "code": "invalid_document_id",
        "type": "invalid_request",
        "link": "https://docs.meilisearch.com/errors#invalid_document_id"
      },
      "duration": "[duration]",
      "enqueuedAt": "[date]",
      "startedAt": "[date]",
      "finishedAt": "[date]"
    }
    "###);
}

#[actix_rt::test]
async fn error_add_documents_missing_document_id() {
    let server = Server::new().await;
    let index = server.index("test");
    index.create(Some("docid")).await;
    let documents = json!([
        {
            "id": "11",
            "content": "foobar"
        }
    ]);
    index.add_documents(documents, None).await;
    index.wait_task(1).await;
    let (response, code) = index.get_task(1).await;
    snapshot!(code, @"200 OK");
    snapshot!(json_string!(response, { ".duration" => "[duration]", ".enqueuedAt" => "[date]", ".startedAt" => "[date]", ".finishedAt" => "[date]" }),
        @r###"
    {
      "uid": 1,
      "indexUid": "test",
      "status": "failed",
      "type": "documentAdditionOrUpdate",
      "canceledBy": null,
      "details": {
        "receivedDocuments": 1,
        "indexedDocuments": 1
      },
      "error": {
        "message": "Document doesn't have a `docid` attribute: `{\"id\":\"11\",\"content\":\"foobar\"}`.",
        "code": "missing_document_id",
        "type": "invalid_request",
        "link": "https://docs.meilisearch.com/errors#missing_document_id"
      },
      "duration": "[duration]",
      "enqueuedAt": "[date]",
      "startedAt": "[date]",
      "finishedAt": "[date]"
    }
    "###);
}

#[actix_rt::test]
#[ignore] // // TODO: Fix in an other PR: this does not provoke any error.
async fn error_document_field_limit_reached() {
    let server = Server::new().await;
    let index = server.index("test");

    index.create(Some("id")).await;

    let mut big_object = std::collections::HashMap::new();
    big_object.insert("id".to_owned(), "wow");
    for i in 0..65535 {
        let key = i.to_string();
        big_object.insert(key, "I am a text!");
    }

    let documents = json!([big_object]);

    let (_response, code) = index.update_documents(documents, Some("id")).await;
    snapshot!(code, @"202");

    index.wait_task(0).await;
    let (response, code) = index.get_task(0).await;
    snapshot!(code, @"200");
    // Documents without a primary key are not accepted.
    snapshot!(json_string!(response, { ".duration" => "[duration]", ".enqueuedAt" => "[date]", ".startedAt" => "[date]", ".finishedAt" => "[date]" }),
        @"");
}

#[actix_rt::test]
async fn add_documents_invalid_geo_field() {
    let server = Server::new().await;
    let index = server.index("test");
    index.create(Some("id")).await;
    index.update_settings(json!({"sortableAttributes": ["_geo"]})).await;

    let documents = json!([
        {
            "id": "11",
            "_geo": "foobar"
        }
    ]);

    index.add_documents(documents, None).await;
    index.wait_task(2).await;
    let (response, code) = index.get_task(2).await;
    snapshot!(code, @"200 OK");
    snapshot!(json_string!(response, { ".duration" => "[duration]", ".enqueuedAt" => "[date]", ".startedAt" => "[date]", ".finishedAt" => "[date]" }),
        @r###"
    {
      "uid": 2,
      "indexUid": "test",
      "status": "failed",
      "type": "documentAdditionOrUpdate",
      "canceledBy": null,
      "details": {
        "receivedDocuments": 1,
        "indexedDocuments": 1
      },
      "error": {
        "message": "The `_geo` field in the document with the id: `11` is not an object. Was expecting an object with the `_geo.lat` and `_geo.lng` fields but instead got `\"foobar\"`.",
        "code": "invalid_document_geo_field",
        "type": "invalid_request",
        "link": "https://docs.meilisearch.com/errors#invalid_document_geo_field"
      },
      "duration": "[duration]",
      "enqueuedAt": "[date]",
      "startedAt": "[date]",
      "finishedAt": "[date]"
    }
    "###);
}

#[actix_rt::test]
async fn error_add_documents_payload_size() {
    let server = Server::new().await;
    let index = server.index("test");
    index.create(Some("id")).await;
    let document = json!(
        {
            "id": "11",
            "content": "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Donec metus erat, consequat in blandit venenatis, ultricies eu ipsum. Etiam luctus elit et mollis ultrices. Nam turpis risus, dictum non eros in, eleifend feugiat elit. Morbi non dolor pulvinar, sagittis mi sed, ultricies lorem. Nulla ultricies sem metus. Donec at suscipit quam, sed elementum mi. Suspendisse potenti. Fusce pharetra turpis tortor, sed eleifend odio dapibus ut. Nulla facilisi. Suspendisse elementum, dui eget aliquet dignissim, ex tellus aliquam nisl, at eleifend nisl metus tempus diam. Mauris fermentum sollicitudin efficitur. Donec dignissim est vitae elit finibus faucibus"
        }
    );
    let documents: Vec<_> = (0..16000).into_iter().map(|_| document.clone()).collect();
    let documents = json!(documents);
    let (response, code) = index.add_documents(documents, None).await;

    snapshot!(code, @"413 Payload Too Large");
    snapshot!(json_string!(response, { ".duration" => "[duration]", ".enqueuedAt" => "[date]", ".startedAt" => "[date]", ".finishedAt" => "[date]" }),
        @r###"
    {
      "message": "The provided payload reached the size limit.",
      "code": "payload_too_large",
      "type": "invalid_request",
      "link": "https://docs.meilisearch.com/errors#payload_too_large"
    }
    "###);
}

#[actix_rt::test]
async fn error_primary_key_inference() {
    let server = Server::new().await;
    let index = server.index("test");

    let documents = json!([
        {
            "title": "11",
            "desc": "foobar"
        }
    ]);

    index.add_documents(documents, None).await;
    index.wait_task(0).await;
    let (response, code) = index.get_task(0).await;
    assert_eq!(code, 200);

    snapshot!(json_string!(response, { ".duration" => "[duration]", ".enqueuedAt" => "[date]", ".startedAt" => "[date]", ".finishedAt" => "[date]" }),
    @r###"
    {
      "uid": 0,
      "indexUid": "test",
      "status": "failed",
      "type": "documentAdditionOrUpdate",
      "canceledBy": null,
      "details": {
        "receivedDocuments": 1,
        "indexedDocuments": 1
      },
      "error": {
        "message": "The primary key inference failed as the engine did not find any field ending with `id` in its name. Please specify the primary key manually using the `primaryKey` query parameter.",
        "code": "index_primary_key_no_candidate_found",
        "type": "invalid_request",
        "link": "https://docs.meilisearch.com/errors#index_primary_key_no_candidate_found"
      },
      "duration": "[duration]",
      "enqueuedAt": "[date]",
      "startedAt": "[date]",
      "finishedAt": "[date]"
    }
    "###);

    let documents = json!([
        {
            "primary_id": "12",
            "object_id": "42",
            "id": "124",
            "title": "11",
            "desc": "foobar"
        }
    ]);

    index.add_documents(documents, None).await;
    index.wait_task(1).await;
    let (response, code) = index.get_task(1).await;
    assert_eq!(code, 200);

    snapshot!(json_string!(response, { ".duration" => "[duration]", ".enqueuedAt" => "[date]", ".startedAt" => "[date]", ".finishedAt" => "[date]" }),
    @r###"
    {
      "uid": 1,
      "indexUid": "test",
      "status": "failed",
      "type": "documentAdditionOrUpdate",
      "canceledBy": null,
      "details": {
        "receivedDocuments": 1,
        "indexedDocuments": 1
      },
      "error": {
        "message": "The primary key inference failed as the engine found 3 fields ending with `id` in their names: 'id' and 'object_id'. Please specify the primary key manually using the `primaryKey` query parameter.",
        "code": "index_primary_key_multiple_candidates_found",
        "type": "invalid_request",
        "link": "https://docs.meilisearch.com/errors#index_primary_key_multiple_candidates_found"
      },
      "duration": "[duration]",
      "enqueuedAt": "[date]",
      "startedAt": "[date]",
      "finishedAt": "[date]"
    }
    "###);

    let documents = json!([
        {
            "primary_id": "12",
            "title": "11",
            "desc": "foobar"
        }
    ]);

    index.add_documents(documents, None).await;
    index.wait_task(2).await;
    let (response, code) = index.get_task(2).await;
    assert_eq!(code, 200);

    snapshot!(json_string!(response, { ".duration" => "[duration]", ".enqueuedAt" => "[date]", ".startedAt" => "[date]", ".finishedAt" => "[date]" }),
    @r###"
    {
      "uid": 2,
      "indexUid": "test",
      "status": "succeeded",
      "type": "documentAdditionOrUpdate",
      "canceledBy": null,
      "details": {
        "receivedDocuments": 1,
        "indexedDocuments": 1
      },
      "error": null,
      "duration": "[duration]",
      "enqueuedAt": "[date]",
      "startedAt": "[date]",
      "finishedAt": "[date]"
    }
    "###);
}

#[actix_rt::test]
async fn add_documents_with_primary_key_twice() {
    let server = Server::new().await;
    let index = server.index("test");

    let documents = json!([
        {
            "title": "11",
            "desc": "foobar"
        }
    ]);

    index.add_documents(documents.clone(), Some("title")).await;
    index.wait_task(0).await;
    let (response, _code) = index.get_task(0).await;
    assert_eq!(response["status"], "succeeded");

    index.add_documents(documents, Some("title")).await;
    index.wait_task(1).await;
    let (response, _code) = index.get_task(1).await;
    assert_eq!(response["status"], "succeeded");
}

#[actix_rt::test]
async fn batch_several_documents_addition() {
    let server = Server::new().await;
    let index = server.index("test");

    let mut documents: Vec<_> = (0..150usize)
        .into_iter()
        .map(|id| {
            json!(
                {
                    "id": id,
                    "title": "foo",
                    "desc": "bar"
                }
            )
        })
        .collect();

    documents[100] = json!({"title": "error", "desc": "error"});

    // enqueue batch of documents
    let mut waiter = Vec::new();
    for chunk in documents.chunks(30) {
        waiter.push(index.add_documents(json!(chunk), Some("id")));
    }

    // wait first batch of documents to finish
    futures::future::join_all(waiter).await;
    index.wait_task(4).await;

    // run a second completely failing batch
    documents[40] = json!({"title": "error", "desc": "error"});
    documents[70] = json!({"title": "error", "desc": "error"});
    documents[130] = json!({"title": "error", "desc": "error"});
    let mut waiter = Vec::new();
    for chunk in documents.chunks(30) {
        waiter.push(index.add_documents(json!(chunk), Some("id")));
    }
    // wait second batch of documents to finish
    futures::future::join_all(waiter).await;
    index.wait_task(9).await;

    let (response, _code) = index.filtered_tasks(&[], &["failed"], &[]).await;

    // Check if only the 6th task failed
    println!("{}", &response);
    assert_eq!(response["results"].as_array().unwrap().len(), 5);

    // Check if there are exactly 120 documents (150 - 30) in the index;
    let (response, code) = index
        .get_all_documents(GetAllDocumentsOptions { limit: Some(200), ..Default::default() })
        .await;
    assert_eq!(code, 200, "failed with `{}`", response);
    assert_eq!(response["results"].as_array().unwrap().len(), 120);
}
