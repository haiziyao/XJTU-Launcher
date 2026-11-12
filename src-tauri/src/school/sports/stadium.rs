//! Sports venue (场馆) list, blacklist check and orders.
//!
//! Ordering is a write flow: check the blacklist, load the details, show a
//! preview, ask the user, then call `check_order`. Venue verification
//! (`getSubOrderDetails`) is an on-site staff action and is not exposed by the
//! campus UI.
use super::{data, Session};
use serde_json::Value;

const LIST: &str = "bdlp_h5_fitness_test/public/index.php/index/Stadium/getStadiumList";
const CHECK_BLACK: &str = "bdlp_h5_fitness_test/public/index.php/index/Stadium/checkBlack";
const ORDER_DETAILS: &str = "bdlp_h5_fitness_test/public/index.php/index/stadium/orderDetails";
const SCAN_AUTH: &str = "bdlp_h5_fitness_test/public/index.php/index/stadium/getScanAuth";
const CHECK_ORDER: &str = "bdlp_h5_fitness_test/public/index.php/index/stadium/checkOrder";
const SUB_ORDER: &str = "bdlp_h5_fitness_test/public/index.php/index/stadium/getSubOrderDetails";

pub(crate) struct Client<'a> {
    pub(super) session: &'a Session,
}

impl Client<'_> {
    /// Venues with their opening information.
    pub(crate) async fn list(&self) -> Result<Value, String> {
        let value = self.session.post(LIST, &[]).await?;
        Ok(data(&value))
    }

    /// Whether the account is barred from booking.
    pub(crate) async fn check_black(&self) -> Result<Value, String> {
        let value = self.session.post(CHECK_BLACK, &[]).await?;
        Ok(data(&value))
    }

    /// Order detail by order id or order number.
    pub(crate) async fn order_details(
        &self,
        order_id: Option<&str>,
        order_num: Option<&str>,
    ) -> Result<Value, String> {
        if order_id.unwrap_or("").is_empty() && order_num.unwrap_or("").is_empty() {
            return Err("需要订单编号".into());
        }
        let value = self
            .session
            .post(
                ORDER_DETAILS,
                &[
                    ("order_id", order_id.unwrap_or("")),
                    ("order_num", order_num.unwrap_or("")),
                ],
            )
            .await?;
        Ok(data(&value))
    }

    /// Whether the account may scan venue codes; `data.permission == 1`.
    pub(crate) async fn scan_auth(&self) -> Result<Value, String> {
        let value = self.session.post(SCAN_AUTH, &[]).await?;
        Ok(data(&value))
    }

    /// Write: submit an order. Requires a preview and explicit confirmation.
    pub(crate) async fn check_order(
        &self,
        order_id: &str,
        details_id: &str,
        school_id: &str,
    ) -> Result<Value, String> {
        let value = self
            .session
            .post(
                CHECK_ORDER,
                &[
                    ("order_id", order_id),
                    ("details_id", details_id),
                    ("school_id", school_id),
                ],
            )
            .await?;
        Ok(data(&value))
    }

    /// Write: on-site verification of a sub-order. Staff-only in practice.
    pub(crate) async fn verify_sub_order(
        &self,
        details_id: &str,
        school_id: &str,
    ) -> Result<Value, String> {
        let value = self
            .session
            .post(
                SUB_ORDER,
                &[("details_id", details_id), ("school_id", school_id)],
            )
            .await?;
        Ok(data(&value))
    }
}
