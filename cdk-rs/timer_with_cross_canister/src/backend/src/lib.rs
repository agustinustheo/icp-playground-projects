use candid::{CandidType, Deserialize, Principal};
use ic_cdk::api::call::CallResult;
use ic_cdk::call;
use ic_cdk_macros::*;
use serde::Serialize;
use std::cell::RefCell;
use std::collections::BTreeMap;

type UserStore = BTreeMap<usize, User>;

thread_local! {
    static USERS: RefCell<UserStore> = RefCell::default();
    static INTERVAL_IN_SECONDS: RefCell<u64> = RefCell::default();
    static TIMERS: RefCell<ic_cdk_timers::TimerId> = RefCell::default();
}

#[derive(CandidType, Deserialize, Serialize, Clone)]
struct User {
    principal: Principal,
    cash: u64,
}

#[derive(CandidType, Deserialize, Serialize)]
struct Error {
    message: String,
}

const LEDGER_CANISTER_ID: &str = "ryjl3-tyaaa-aaaaa-aaaba-cai";

async fn update_user_balances(users: RefCell<UserStore>) {
    let mut users = users.borrow_mut();
    for user in users.values_mut() {
        let ledger_principal = Principal::from_text(LEDGER_CANISTER_ID).expect("Invalid principal");
        let call_result: CallResult<(u64,)> = call(
            ledger_principal,
            "icrc1_balance_of",
            (user.principal.clone(),),
        )
        .await;

        let response = call_result.map_err(|e| {
            println!("An error occurred: {:?}", e);
        });
        match response {
            Ok(response) => user.cash = response.0, // Set value of icrc1 balance for each user
            _ => println!("An error occurred while getting balance"),
        };
    }
}

#[ic_cdk::init]
fn init() {
    let seconds = 3;
    INTERVAL_IN_SECONDS.with(|interval_ref| {
        interval_ref.replace(seconds);
    });

    let interval = std::time::Duration::from_secs(seconds);
    ic_cdk::println!("Starting a periodic task with interval {:?}", interval);
    let timer_id = ic_cdk_timers::set_timer_interval(interval, || {
        USERS.with(move |users| {
            // Trigger the async update function
            ic_cdk::spawn(update_user_balances(users.clone()));
        });
    });
    TIMERS.with(|timers_ref| {
        timers_ref.replace(timer_id);
    });
}

#[query]
fn get_interval() -> Result<u64, Error> {
    INTERVAL_IN_SECONDS.with(|interval_ref| Ok(interval_ref.borrow().clone()))
}

#[update]
fn set_interval(seconds: u64) -> Result<u64, Error> {
    TIMERS.with(|timers_ref| {
        let timer_id = timers_ref.borrow().clone();
        ic_cdk_timers::clear_timer(timer_id);
    });

    let interval = std::time::Duration::from_secs(seconds);
    ic_cdk::println!("Starting a periodic task with interval {:?}", interval);
    let new_timer_id = ic_cdk_timers::set_timer_interval(interval, || {
        USERS.with(move |users| {
            // Trigger the async update function
            ic_cdk::spawn(update_user_balances(users.clone()));
        });
    });
    TIMERS.with(|timers_ref| {
        timers_ref.replace(new_timer_id);
    });

    INTERVAL_IN_SECONDS.with(|seconds_ref| {
        seconds_ref.replace(seconds);
    });

    Ok(seconds)
}

#[update]
async fn add_user(principal: String) -> Result<String, Error> {
    USERS.with(|users| {
        let mut users = users.borrow_mut();

        let new_id = users.len() + 1; // Simple way to generate a new ID
        let user = User {
            principal: Principal::from_text(&principal).unwrap(),
            cash: 1,
        };
        users.insert(new_id, user);

        let principals: Vec<User> = users.values().cloned().collect();
        let res = serde_json::to_string(&principals).unwrap();
        Ok(res)
    })
}

#[query]
fn get_users() -> Result<String, String> {
    USERS.with(|users| {
        let users = users.borrow();

        let principals: Vec<User> = users.values().cloned().collect();
        let res = serde_json::to_string(&principals).unwrap();
        Ok(res)
    })
}

#[query]
fn search_users(query: String) -> Result<String, String> {
    USERS.with(|users| {
        let users = users.borrow();

        // Filter the users whose principals contain the query string
        let filtered_principals: Vec<User> = users
            .values()
            .filter(|user| {
                user.principal
                    .to_string()
                    .to_lowercase()
                    .contains(&query.to_lowercase())
            })
            .cloned()
            .collect();

        // Convert the filtered list of principals to a JSON string
        match serde_json::to_string(&filtered_principals) {
            Ok(res) => Ok(res),
            Err(e) => Err(format!("Failed to serialize principals: {}", e)),
        }
    })
}

// Enable Candid export
ic_cdk::export_candid!();
