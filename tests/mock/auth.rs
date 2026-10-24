use dkregistry::{errors::Error, v2::Client};
use mockito::{mock, Matcher, Mock};
use tokio::runtime::Runtime;

const REGISTRY_SCOPE: &str = "repository:audit/release:pull";
const BASIC_AUTHORIZATION: &str = "Basic c3ludGhldGljLXVzZXI6c3ludGhldGljLXBhc3N3b3Jk";

#[derive(Clone, Copy)]
enum ExpectedResult {
    Authenticated(&'static str),
    Rejected(ExpectedError),
}

#[derive(Clone, Copy)]
enum ExpectedError {
    Decode,
    InvalidAuthToken(&'static str),
    NoTokenReceived,
    Unauthorized,
}

struct AuthCase {
    name: &'static str,
    token_response: &'static str,
    token_status: usize,
    expected: ExpectedResult,
}

fn mock_token_exchange(case: &AuthCase) -> (Mock, Mock, Mock) {
    let realm = format!("{}/oauth2/token", mockito::server_url());
    let challenge_header = format!(r#"Bearer realm="{realm}",service="audit-registry""#);

    let challenge = mock("GET", "/v2/")
        .match_header("authorization", Matcher::Missing)
        .with_status(401)
        .with_header("www-authenticate", &challenge_header)
        .expect(1)
        .create();

    let token_response = mock("GET", "/oauth2/token")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("service".into(), "audit-registry".into()),
            Matcher::UrlEncoded("scope".into(), REGISTRY_SCOPE.into()),
        ]))
        .match_header("authorization", BASIC_AUTHORIZATION)
        .with_status(case.token_status)
        .with_header("content-type", "application/json")
        .with_body(case.token_response)
        .expect(1)
        .create();

    let authenticated_request = match case.expected {
        ExpectedResult::Authenticated(token) => {
            let expected_authorization = format!("Bearer {token}");
            mock("GET", "/v2/")
                .match_header("authorization", expected_authorization.as_str())
                .with_status(200)
                .expect(1)
                .create()
        }
        ExpectedResult::Rejected(_) => mock("GET", "/v2/")
            .match_header("authorization", Matcher::Any)
            .with_status(200)
            .expect_at_most(0)
            .create(),
    };

    (challenge, token_response, authenticated_request)
}

fn assert_expected_error(error: Error, expected: ExpectedError, case_name: &str) {
    let matches = match expected {
        ExpectedError::Decode => matches!(&error, Error::Reqwest(_)),
        ExpectedError::InvalidAuthToken(expected) => {
            matches!(&error, Error::InvalidAuthToken(actual) if actual.as_str() == expected)
        }
        ExpectedError::NoTokenReceived => matches!(&error, Error::NoTokenReceived),
        ExpectedError::Unauthorized => {
            matches!(&error, Error::UnexpectedHttpStatus(status) if status.as_u16() == 401)
        }
    };
    assert!(
        matches,
        "{}: unexpected authentication error: {:?}",
        case_name, error
    );
}

fn run_auth_case(runtime: &Runtime, case: AuthCase) {
    let (challenge, token_response, authenticated_request) = mock_token_exchange(&case);
    let registry = mockito::server_address().to_string();
    let client = Client::configure()
        .registry(&registry)
        .insecure_registry(true)
        .username(Some("synthetic-user".to_string()))
        .password(Some("synthetic-password".to_string()))
        .build()
        .unwrap();

    let result = runtime.block_on(client.authenticate(&[REGISTRY_SCOPE]));
    match case.expected {
        ExpectedResult::Authenticated(expected_token) => {
            let client = result.unwrap_or_else(|error| {
                panic!(
                    "{}: expected authentication with {:?}, got {:?}",
                    case.name, expected_token, error
                )
            });
            assert!(
                runtime.block_on(client.is_auth()).unwrap(),
                "{}: authenticated client was rejected by the registry",
                case.name
            );
        }
        ExpectedResult::Rejected(expected_error) => {
            let error = result.unwrap_err();
            assert_expected_error(error, expected_error, case.name);
        }
    }

    challenge.assert();
    token_response.assert();
    authenticated_request.assert();
}

#[test]
fn bearer_auth_response_schemas() {
    let runtime = Runtime::new().unwrap();
    let cases = [
        AuthCase {
            name: "Docker token field",
            token_response: r#"{"token":"docker-token","expires_in":300}"#,
            token_status: 200,
            expected: ExpectedResult::Authenticated("docker-token"),
        },
        AuthCase {
            name: "Azure access_token field",
            token_response: r#"{"access_token":"azure-token","expires_in":300}"#,
            token_status: 200,
            expected: ExpectedResult::Authenticated("azure-token"),
        },
        AuthCase {
            name: "token takes precedence when both fields are present",
            token_response: r#"{"token":"preferred-token","access_token":"alternate-token"}"#,
            token_status: 200,
            expected: ExpectedResult::Authenticated("preferred-token"),
        },
        AuthCase {
            name: "null token falls back to access_token",
            token_response: r#"{"token":null,"access_token":"azure-token"}"#,
            token_status: 200,
            expected: ExpectedResult::Authenticated("azure-token"),
        },
        AuthCase {
            name: "missing token fields are rejected",
            token_response: "{}",
            token_status: 200,
            expected: ExpectedResult::Rejected(ExpectedError::NoTokenReceived),
        },
        AuthCase {
            name: "null token fields are rejected",
            token_response: r#"{"token":null,"access_token":null}"#,
            token_status: 200,
            expected: ExpectedResult::Rejected(ExpectedError::NoTokenReceived),
        },
        AuthCase {
            name: "empty token is rejected",
            token_response: r#"{"token":""}"#,
            token_status: 200,
            expected: ExpectedResult::Rejected(ExpectedError::InvalidAuthToken("")),
        },
        AuthCase {
            name: "empty access_token is rejected",
            token_response: r#"{"access_token":""}"#,
            token_status: 200,
            expected: ExpectedResult::Rejected(ExpectedError::InvalidAuthToken("")),
        },
        AuthCase {
            name: "unauthenticated token is rejected",
            token_response: r#"{"token":"unauthenticated"}"#,
            token_status: 200,
            expected: ExpectedResult::Rejected(ExpectedError::InvalidAuthToken("unauthenticated")),
        },
        AuthCase {
            name: "unauthenticated access_token is rejected",
            token_response: r#"{"access_token":"unauthenticated"}"#,
            token_status: 200,
            expected: ExpectedResult::Rejected(ExpectedError::InvalidAuthToken("unauthenticated")),
        },
        AuthCase {
            name: "empty token does not fall back to access_token",
            token_response: r#"{"token":"","access_token":"alternate-token"}"#,
            token_status: 200,
            expected: ExpectedResult::Rejected(ExpectedError::InvalidAuthToken("")),
        },
        AuthCase {
            name: "unauthenticated token does not fall back to access_token",
            token_response: r#"{"token":"unauthenticated","access_token":"alternate-token"}"#,
            token_status: 200,
            expected: ExpectedResult::Rejected(ExpectedError::InvalidAuthToken("unauthenticated")),
        },
        AuthCase {
            name: "numeric token is rejected during decoding",
            token_response: r#"{"token":123}"#,
            token_status: 200,
            expected: ExpectedResult::Rejected(ExpectedError::Decode),
        },
        AuthCase {
            name: "numeric access_token is rejected during decoding",
            token_response: r#"{"access_token":123}"#,
            token_status: 200,
            expected: ExpectedResult::Rejected(ExpectedError::Decode),
        },
        AuthCase {
            name: "malformed secondary field is rejected even with a token",
            token_response: r#"{"token":"docker-token","access_token":123}"#,
            token_status: 200,
            expected: ExpectedResult::Rejected(ExpectedError::Decode),
        },
        AuthCase {
            name: "token endpoint HTTP errors are rejected",
            token_response: r#"{"error":"unauthorized"}"#,
            token_status: 401,
            expected: ExpectedResult::Rejected(ExpectedError::Unauthorized),
        },
    ];

    for case in cases {
        run_auth_case(&runtime, case);
    }
}
