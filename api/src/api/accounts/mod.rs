pub mod handlers;
pub mod models;
pub mod transform;
pub mod util;

use actix_web::web;

use crate::{
    api::accounts,
    traits::{AccountsRepository, TransactionsRepository},
};

pub fn configure_accounts_api<AR: AccountsRepository, TR: TransactionsRepository>(
    cfg: &mut web::ServiceConfig,
) {
    cfg.service(
        web::scope("/api/customers/{customer_id}/accounts")
            .service(
                web::resource("")
                    .route(web::post().to(accounts::handlers::create_account::<AR, TR>))
                    .route(web::get().to(accounts::handlers::find_accounts::<AR, TR>)),
            )
            .service(
                web::resource("/{account_id}")
                    .route(web::get().to(accounts::handlers::get_account::<AR, TR>))
                    .route(web::delete().to(accounts::handlers::delete_account::<AR, TR>)),
            ),
    );
}
