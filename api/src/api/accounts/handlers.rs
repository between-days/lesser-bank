use actix_web::http::header::ContentType;
use actix_web::web::{Data, Path, Query};
use actix_web::{web, HttpResponse};

use super::models::{AccountRest, AccountsRest, FindAccountQueryRest, NewAccountRest};

use crate::api::accounts::util::get_random_account_number;
use crate::api::error::ApiError;
use crate::error::RepoError;
use crate::models::account::{Account, FindAccountQuery, NewAccount};
use crate::traits::{RepoCreate, RepoDeleteById, RepoFind, RepoGetById};

pub async fn create_account<AR>(
    accounts_repo: Data<AR>,
    path: Path<i32>,
    payload: web::Json<NewAccountRest>,
) -> Result<HttpResponse, actix_web::Error>
where
    AR: RepoCreate<Account, NewAccount>,
{
    let customer_id = path.into_inner();

    let mut new_account: NewAccount = payload.into_inner().into();

    if new_account.customer_id != customer_id {
        return Err(ApiError::Unauthorized.into());
    }

    // completely random for now
    let account_number = get_random_account_number();
    new_account.account_number = account_number;

    println!(
        "Trying to create {:?} account for customer {}",
        new_account.account_type, customer_id
    );

    let account = web::block(move || accounts_repo.create(new_account))
        .await
        .map_err(|_| ApiError::InternalError)?
        .map_err(|_| ApiError::InternalError)?;

    Ok(HttpResponse::Created()
        .insert_header(ContentType::json())
        .json(web::Json::<AccountRest>((&account).into())))
}

pub async fn find_accounts<AR>(
    accounts_repo: Data<AR>,
    path: Path<i32>,
    query: Query<FindAccountQueryRest>,
) -> Result<HttpResponse, actix_web::Error>
where
    AR: RepoFind<Account, FindAccountQuery>,
{
    let customer_id = path.into_inner();

    let query = FindAccountQuery {
        account_id: query.account_id,
        customer_id,
        account_number: query.account_number.clone(),
    };

    if query.customer_id != customer_id {
        return Err(ApiError::Unauthorized.into());
    }

    println!("Trying to get accounts for customer {}", customer_id);

    let accounts = web::block(move || accounts_repo.find(query))
        .await
        .map_err(|_| ApiError::InternalError)?
        .map_err(|_| ApiError::InternalError)?;

    // enforced query on customer_id so should always be fine, but safe > sorry
    for acc in accounts.iter() {
        if acc.customer_id != customer_id {
            return Err(ApiError::BadRequest.into());
        }
    }

    println!("Got accounts for customer {}", customer_id);

    Ok(HttpResponse::Ok()
        .insert_header(ContentType::json())
        .json(web::Json::<AccountsRest>(accounts.into())))
}

pub async fn get_account<AR>(
    accounts_repo: Data<AR>,
    path: Path<(i32, i32)>,
) -> Result<HttpResponse, actix_web::Error>
where
    AR: RepoGetById<Account>,
{
    let (customer_id, account_id) = path.into_inner();

    println!(
        "Trying to get account {}, for customer {}",
        account_id, customer_id
    );

    let account = web::block(move || accounts_repo.get_by_id(account_id))
        .await
        .map_err(|_| ApiError::InternalError)?
        .map_err(|err| match err {
            RepoError::NotFound => ApiError::NotFound,
            _ => ApiError::InternalError,
        })?;

    if account.customer_id != customer_id {
        return Err(ApiError::Unauthorized.into());
    }

    Ok(HttpResponse::Ok()
        .insert_header(ContentType::json())
        .json(web::Json::<AccountRest>((&account).into())))
}

pub async fn delete_account<AR>(
    accounts_repo: Data<AR>,
    path: Path<(i32, i32)>,
) -> Result<HttpResponse, actix_web::Error>
where
    AR: RepoGetById<Account> + RepoDeleteById<Account>,
{
    let (customer_id, account_id) = path.into_inner();

    println!(
        "Trying to delete account {}, for customer {}",
        account_id, customer_id
    );

    web::block(move || {
        let get_acc_res = accounts_repo.get_by_id(account_id);

        let check_account_error: Option<ApiError> = match get_acc_res {
            Ok(acc) => {
                if acc.customer_id != customer_id {
                    Some(ApiError::Unauthorized)
                } else {
                    None
                }
            }
            Err(RepoError::NotFound) => None,
            _ => Some(ApiError::InternalError),
        };

        if check_account_error.is_some() {
            let cc = check_account_error.unwrap();
            return Err(cc);
        }

        accounts_repo
            .delete_by_id(account_id)
            .map_err(|_| ApiError::InternalError)
    })
    .await
    .map_err(|_| ApiError::InternalError)?
    .map_err(|err| err)?;

    Ok(HttpResponse::NoContent().body(""))
}

#[cfg(test)]
mod tests {
    use std::vec;

    use crate::{
        api::{
            accounts::{
                handlers::{create_account, delete_account, find_accounts, get_account},
                models::{
                    AccountRest, AccountStatusRest, AccountTypeRest, AccountsRest, NewAccountRest,
                },
            },
            error::ApiError,
        },
        error::RepoError,
        models::account::{Account, AccountStatus, AccountType, FindAccountQuery, NewAccount},
        traits::{MockRepoCreate, MockRepoFind, MockRepoGetById, RepoDeleteById, RepoGetById},
    };

    use actix_web::{
        body::to_bytes,
        http::StatusCode,
        test,
        web::{self, Data, Json},
        App,
    };
    use chrono::{NaiveDate, NaiveDateTime};
    use mockall::{mock, predicate::eq, Sequence};

    #[actix_web::test]
    async fn test_create_account_success() {
        let customer_id = 1;
        let account_id = 2;

        let dt: NaiveDateTime = NaiveDate::from_ymd_opt(2016, 7, 8)
            .unwrap()
            .and_hms_opt(9, 10, 11)
            .unwrap();

        let mut mock_accounts_repo = MockRepoCreate::<Account, NewAccount>::new();
        mock_accounts_repo
            .expect_create()
            .withf(move |acc| {
                acc.customer_id == customer_id
                    && acc.balance_cents == 34343
                    && acc.account_type == AccountType::Savings
                    && acc.account_name == Some("abc".to_string())
                    && acc.available_balance_cents == 3444
            })
            .times(1)
            .returning(move |_| {
                Ok(Account {
                    id: account_id,
                    customer_id,
                    balance_cents: 13424234234,
                    account_type: AccountType::Savings,
                    date_opened: dt,
                    account_status: AccountStatus::Active,
                    account_name: Some("abc".to_string()),
                    account_number: "012345678".to_string(),
                    available_balance_cents: 3444,
                    bsb: "123456".to_string(),
                })
            });

        let mut app = test::init_service(
            App::new().app_data(Data::new(mock_accounts_repo)).service(
                web::resource("/customers/{customer_id}/accounts")
                    .route(web::post().to(create_account::<MockRepoCreate<Account, NewAccount>>)),
            ),
        )
        .await;

        let testpayload = NewAccountRest {
            customer_id,
            balance_cents: 34343,
            available_balance_cents: 3444,
            account_type: AccountTypeRest::Savings,
            account_name: Some("abc".to_string()),
        };

        let resp = test::TestRequest::post()
            .uri("/customers/1/accounts")
            .append_header(("Content-Type", "application/json"))
            .set_payload(serde_json::to_string(&testpayload).unwrap())
            .send_request(&mut app)
            .await;

        let actual_status = resp.status();
        assert_eq!(StatusCode::CREATED, actual_status);

        let expected_account = AccountRest {
            id: account_id,
            customer_id,
            balance_cents: 13424234234,
            account_type: AccountTypeRest::Savings,
            date_opened: dt.to_string(),
            account_status: AccountStatusRest::Active,
            account_name: Some("abc".to_string()),
            account_number: "012345678".to_string(),
            available_balance_cents: 3444,
            bsb: "123456".to_string(),
        };

        let actual_account: AccountRest = test::read_body_json(resp).await;
        assert_eq!(expected_account, actual_account)
    }

    #[actix_web::test]
    async fn test_create_account_internal_error() {
        let customer_id = 5;

        let new_account = NewAccount {
            customer_id,
            balance_cents: 13424234234,
            account_type: AccountType::Savings,
            account_name: Some("abc".to_string()),
            available_balance_cents: 34343,
            account_number: "".to_string(),
        };

        let mut mock_accounts_repo = MockRepoCreate::<Account, NewAccount>::new();
        mock_accounts_repo
            .expect_create()
            .with(eq(new_account))
            .times(1)
            .returning(move |_| Err(RepoError::Other));

        let res = create_account(
            Data::new(mock_accounts_repo),
            customer_id.into(),
            Json(NewAccountRest {
                customer_id,
                balance_cents: 13424234234,
                available_balance_cents: 3444,
                account_type: AccountTypeRest::Savings,
                account_name: Some("abc".to_string()),
            }),
        )
        .await;

        assert!(res.is_err_and(|e| { e.to_string() == ApiError::InternalError.to_string() }));
    }

    #[actix_web::test]
    async fn test_find_accounts_by_customer_id_success() {
        let customer_id = 1;

        let expected_query = FindAccountQuery {
            account_id: None,
            customer_id,
            account_number: None,
        };

        let mut mock_accounts_repo = MockRepoFind::<Account, FindAccountQuery>::new();
        mock_accounts_repo
            .expect_find()
            .with(eq(expected_query))
            .times(1)
            .returning(move |_| {
                Ok(vec![
                    Account {
                        id: 1,
                        customer_id: 1,
                        balance_cents: 13424234234,
                        account_type: AccountType::Savings,
                        date_opened: NaiveDate::from_ymd_opt(2016, 7, 8)
                            .unwrap()
                            .and_hms_opt(9, 10, 11)
                            .unwrap(),
                        account_status: AccountStatus::Active,
                        account_name: Some("abc".to_string()),
                        available_balance_cents: 34343,
                        account_number: "012345678".to_string(),
                        bsb: "123456".to_string(),
                    },
                    Account {
                        id: 2,
                        customer_id: 1,
                        balance_cents: 13424234234,
                        account_type: AccountType::Savings,
                        date_opened: NaiveDate::from_ymd_opt(2016, 7, 8)
                            .unwrap()
                            .and_hms_opt(9, 10, 11)
                            .unwrap(),
                        account_status: AccountStatus::Active,
                        available_balance_cents: 34343,
                        account_name: Some("abc".to_string()),
                        account_number: "012345678".to_string(),
                        bsb: "123456".to_string(),
                    },
                ])
            });

        let mut app = test::init_service(
            App::new()
                .app_data(Data::new(mock_accounts_repo))
                .service(web::resource("/customers/{customer_id}/accounts").route(
                    web::get().to(find_accounts::<MockRepoFind<Account, FindAccountQuery>>),
                )),
        )
        .await;

        let resp = test::TestRequest::get()
            .uri(format!("/customers/{0}/accounts?customerId={0}", customer_id).as_str())
            .append_header(("Content-Type", "application/json"))
            .send_request(&mut app)
            .await;

        let actual_status = resp.status();
        assert_eq!(StatusCode::OK, actual_status);

        let expected_res = AccountsRest {
            accounts: vec![
                AccountRest {
                    id: 1,
                    customer_id: 1,
                    balance_cents: 13424234234,
                    account_type: AccountTypeRest::Savings,
                    date_opened: "2016-07-08 09:10:11".to_string(),
                    account_status: AccountStatusRest::Active,
                    account_name: Some("abc".to_string()),
                    available_balance_cents: 34343,
                    account_number: "012345678".to_string(),
                    bsb: "123456".to_string(),
                },
                AccountRest {
                    id: 2,
                    customer_id: 1,
                    balance_cents: 13424234234,
                    account_type: AccountTypeRest::Savings,
                    date_opened: "2016-07-08 09:10:11".to_string(),
                    account_status: AccountStatusRest::Active,
                    account_name: Some("abc".to_string()),
                    available_balance_cents: 34343,
                    account_number: "012345678".to_string(),
                    bsb: "123456".to_string(),
                },
            ],
        };

        let actual_res: AccountsRest = test::read_body_json(resp).await;
        assert_eq!(expected_res.accounts[0], actual_res.accounts[0]);
        assert_eq!(expected_res.accounts[1], actual_res.accounts[1]);
    }

    #[actix_web::test]
    async fn test_find_accounts_internal_error() {
        let customer_id = 2;

        let expected_query = FindAccountQuery {
            account_id: None,
            customer_id,
            account_number: None,
        };

        let mut mock_accounts_repo = MockRepoFind::<Account, FindAccountQuery>::new();
        mock_accounts_repo
            .expect_find()
            .with(eq(expected_query))
            .times(1)
            .returning(move |_| Err(RepoError::Other));

        let mut app = test::init_service(
            App::new()
                .app_data(Data::new(mock_accounts_repo))
                .service(web::resource("/customers/{customer_id}/accounts").route(
                    web::get().to(find_accounts::<MockRepoFind<Account, FindAccountQuery>>),
                )),
        )
        .await;

        let resp = test::TestRequest::get()
            .uri(format!("/customers/{0}/accounts?customerId={0}", customer_id).as_str())
            .append_header(("Content-Type", "application/json"))
            .send_request(&mut app)
            .await;

        let actual_status = resp.status();
        assert_eq!(StatusCode::INTERNAL_SERVER_ERROR, actual_status);
    }

    #[actix_web::test]
    async fn test_get_account_success() {
        let customer_id = 1;
        let account_id = 2;

        let mut mock_accounts_repo = MockRepoGetById::<Account>::new();
        mock_accounts_repo
            .expect_get_by_id()
            .with(eq(account_id))
            .times(1)
            .returning(move |_| {
                Ok(Account {
                    id: account_id,
                    customer_id,
                    balance_cents: 13424234234,
                    account_type: AccountType::Savings,
                    date_opened: NaiveDate::from_ymd_opt(2016, 7, 8)
                        .unwrap()
                        .and_hms_opt(9, 10, 11)
                        .unwrap(),
                    account_status: AccountStatus::Active,
                    account_name: Some("abc".to_string()),
                    available_balance_cents: 34343,
                    account_number: "012345678".to_string(),
                    bsb: "123456".to_string(),
                })
            });

        let res = get_account(
            Data::new(mock_accounts_repo),
            (customer_id, account_id).into(),
        )
        .await
        .unwrap();

        let actual_status = res.status();
        let actual_body = to_bytes(res.into_body()).await.unwrap();

        let expected_status = StatusCode::OK;
        let expected_body = serde_json::to_string::<AccountRest>(&AccountRest {
            id: account_id,
            customer_id,
            balance_cents: 13424234234,
            account_type: AccountTypeRest::Savings,
            date_opened: "2016-07-08 09:10:11".to_string(),
            account_status: AccountStatusRest::Active,
            account_name: Some("abc".to_string()),
            available_balance_cents: 34343,
            account_number: "012345678".to_string(),
            bsb: "123456".to_string(),
        })
        .unwrap();

        assert_eq!(expected_status, actual_status);
        assert_eq!(expected_body, actual_body)
    }

    #[actix_web::test]
    async fn test_get_account_internal_error() {
        let customer_id = 1;
        let account_id = 4;

        let mut mock_accounts_repo = MockRepoGetById::<Account>::new();
        mock_accounts_repo
            .expect_get_by_id()
            .with(eq(account_id))
            .times(1)
            .returning(move |_| Err(RepoError::Other));

        let res = get_account(
            Data::new(mock_accounts_repo),
            (customer_id, account_id).into(),
        )
        .await;

        assert!(res.is_err_and(|e| { e.to_string() == ApiError::InternalError.to_string() }));
    }

    #[actix_web::test]
    async fn test_get_account_not_found_error() {
        let customer_id = 1;
        let account_id = 4;

        let mut mock_accounts_repo = MockRepoGetById::<Account>::new();
        mock_accounts_repo
            .expect_get_by_id()
            .with(eq(account_id))
            .times(1)
            .returning(move |_| Err(RepoError::NotFound));

        let res = get_account(
            Data::new(mock_accounts_repo),
            (customer_id, account_id).into(),
        )
        .await;

        assert!(res.is_err_and(|e| { e.to_string() == ApiError::NotFound.to_string() }));
    }

    #[actix_web::test]
    async fn test_get_account_unauthorized_error() {
        let customer_id = 1;
        let wrong_customer_id = 5;
        let account_id = 4;

        let mut mock_accounts_repo = MockRepoGetById::<Account>::new();
        mock_accounts_repo
            .expect_get_by_id()
            .with(eq(account_id))
            .times(1)
            .returning(move |_| {
                Ok(Account {
                    id: account_id,
                    customer_id,
                    balance_cents: 13424234234,
                    account_type: AccountType::Savings,
                    date_opened: NaiveDate::from_ymd_opt(2016, 7, 8)
                        .unwrap()
                        .and_hms_opt(9, 10, 11)
                        .unwrap(),
                    account_status: AccountStatus::Active,
                    account_name: Some("abc".to_string()),
                    available_balance_cents: 34343,
                    account_number: "012345678".to_string(),
                    bsb: "123456".to_string(),
                })
            });

        let res = get_account(
            Data::new(mock_accounts_repo),
            (wrong_customer_id, account_id).into(),
        )
        .await;

        assert!(res.is_err_and(|e| { e.to_string() == ApiError::Unauthorized.to_string() }));
    }

    #[actix_web::test]
    async fn test_delete_account_success() {
        let customer_id = 1;
        let account_id = 2;

        let mut seq = Sequence::new();

        mock! {
            pub AR { }
            impl RepoGetById<Account> for AR {
                fn get_by_id(&self, new: i32) -> Result<Account, RepoError>;
            }
            impl RepoDeleteById<Account> for AR {
                fn delete_by_id(&self, id: i32) -> Result<(), RepoError>;
            }
        };

        let mut mock_accounts_repo = MockAR::new();

        mock_accounts_repo
            .expect_get_by_id()
            .with(eq(account_id))
            .times(1)
            .returning(move |_| {
                Ok(Account {
                    id: account_id,
                    customer_id,
                    balance_cents: 13424234234,
                    account_type: AccountType::Savings,
                    date_opened: NaiveDate::from_ymd_opt(2016, 7, 8)
                        .unwrap()
                        .and_hms_opt(9, 10, 11)
                        .unwrap(),
                    account_status: AccountStatus::Active,
                    account_name: Some("abc".to_string()),
                    available_balance_cents: 34343,
                    account_number: "012345678".to_string(),
                    bsb: "123456".to_string(),
                })
            })
            .in_sequence(&mut seq);

        mock_accounts_repo
            .expect_delete_by_id()
            .with(eq(account_id))
            .times(1)
            .returning(|_| Ok(()))
            .in_sequence(&mut seq);

        let res = delete_account(
            Data::new(mock_accounts_repo),
            (customer_id, account_id).into(),
        )
        .await
        .unwrap();

        let actual_status = res.status();
        let actual_body = to_bytes(res.into_body()).await.unwrap();

        let expected_status = StatusCode::NO_CONTENT;

        assert_eq!(expected_status, actual_status);
        assert_eq!("", actual_body)
    }

    #[actix_web::test]
    async fn test_delete_account_not_found_success() {
        let customer_id = 5;
        let account_id = 2;

        let mut seq = Sequence::new();

        mock! {
            pub AR { }
            impl RepoGetById<Account> for AR {
                fn get_by_id(&self, new: i32) -> Result<Account, RepoError>;
            }
            impl RepoDeleteById<Account> for AR {
                fn delete_by_id(&self, id: i32) -> Result<(), RepoError>;
            }
        };

        let mut mock_accounts_repo = MockAR::new();

        mock_accounts_repo
            .expect_get_by_id()
            .with(eq(account_id))
            .times(1)
            .returning(move |_| Err(crate::error::RepoError::NotFound))
            .in_sequence(&mut seq);

        mock_accounts_repo
            .expect_delete_by_id()
            .with(eq(account_id))
            .times(1)
            .returning(|_| Ok(()))
            .in_sequence(&mut seq);

        let res = delete_account(
            Data::new(mock_accounts_repo),
            (customer_id, account_id).into(),
        )
        .await
        .unwrap();

        let actual_status = res.status();
        let actual_body = to_bytes(res.into_body()).await.unwrap();

        let expected_status = StatusCode::NO_CONTENT;

        assert_eq!(expected_status, actual_status);
        assert_eq!("", actual_body)
    }

    #[actix_web::test]
    async fn test_delete_account_unauthorized_error() {
        let customer_id = 5;
        let account_id = 2;
        let wrong_customer_id = 2;

        mock! {
            pub AR { }
            impl RepoGetById<Account> for AR {
                fn get_by_id(&self, new: i32) -> Result<Account, RepoError>;
            }
            impl RepoDeleteById<Account> for AR {
                fn delete_by_id(&self, id: i32) -> Result<(), RepoError>;
            }
        };

        let mut mock_accounts_repo = MockAR::new();
        mock_accounts_repo
            .expect_get_by_id()
            .with(eq(account_id))
            .times(1)
            .returning(move |_| {
                Ok(Account {
                    id: account_id,
                    customer_id,
                    balance_cents: 13424234234,
                    account_type: AccountType::Savings,
                    date_opened: NaiveDate::from_ymd_opt(2016, 7, 8)
                        .unwrap()
                        .and_hms_opt(9, 10, 11)
                        .unwrap(),
                    account_status: AccountStatus::Active,
                    account_name: Some("abc".to_string()),
                    available_balance_cents: 34343,
                    account_number: "012345678".to_string(),
                    bsb: "123456".to_string(),
                })
            });

        let res = delete_account(
            Data::new(mock_accounts_repo),
            (wrong_customer_id, account_id).into(),
        )
        .await;

        assert!(res.is_err_and(|e| { e.to_string() == ApiError::Unauthorized.to_string() }));
    }
    #[actix_web::test]
    async fn test_delete_account_internal_error() {
        let customer_id = 5;
        let account_id = 2;

        mock! {
            pub AR { }
            impl RepoGetById<Account> for AR {
                fn get_by_id(&self, new: i32) -> Result<Account, RepoError>;
            }
            impl RepoDeleteById<Account> for AR {
                fn delete_by_id(&self, id: i32) -> Result<(), RepoError>;
            }
        };

        let mut mock_accounts_repo = MockAR::new();
        mock_accounts_repo
            .expect_get_by_id()
            .with(eq(account_id))
            .times(1)
            .returning(move |_| Err(RepoError::Other));

        let res = delete_account(
            Data::new(mock_accounts_repo),
            (customer_id, account_id).into(),
        )
        .await;

        assert!(res.is_err_and(|e| { e.to_string() == ApiError::InternalError.to_string() }));
    }
}
