pub mod client;
pub mod model;
mod transport;

use crate::{
    errors::OpenLimitError,
    exchange::Exchange,
    exchange_info::ExchangeInfo,
    model::{
        Asks, Balance, Bids, CancelAllOrdersRequest, CancelOrderRequest, Candle,
        GetHistoricRatesRequest, GetOrderHistoryRequest, GetPriceTickerRequest, Interval,
        Liquidity, OpenLimitOrderRequest, OpenMarketOrderRequest, Order, OrderBookRequest,
        OrderBookResponse, OrderCanceled, Paginator, Side, Ticker, Trade, TradeHistoryRequest,
    },
    shared::{timestamp_to_datetime, Result},
};
use async_trait::async_trait;

use model::{CandleRequestParams, GetOrderRequest};
use std::convert::TryFrom;
use transport::Transport;

#[derive(Clone)]
pub struct Coinbase {
    exchange_info: ExchangeInfo,
    transport: Transport,
}

impl Coinbase {
    pub async fn new(sandbox: bool) -> Self {
        let state = Coinbase {
            exchange_info: ExchangeInfo::new(),
            transport: Transport::new(sandbox).unwrap(),
        };

        state.refresh_market_info().await.unwrap();
        state
    }

    pub async fn with_credential(
        api_key: &str,
        api_secret: &str,
        passphrase: &str,
        sandbox: bool,
    ) -> Self {
        let state = Coinbase {
            exchange_info: ExchangeInfo::new(),
            transport: Transport::with_credential(api_key, api_secret, passphrase, sandbox)
                .unwrap(),
        };

        state.refresh_market_info().await.unwrap();
        state
    }
}

#[async_trait]
impl Exchange for Coinbase {
    type OrderIdType = String;
    type TradeIdType = u64;
    async fn order_book(&self, req: &OrderBookRequest) -> Result<OrderBookResponse> {
        self.book::<model::BookRecordL2>(&req.symbol)
            .await
            .map(Into::into)
    }

    async fn limit_buy(&self, req: &OpenLimitOrderRequest) -> Result<Order<Self::OrderIdType>> {
        Coinbase::limit_buy(self, &req.symbol, req.size, req.price)
            .await
            .map(Into::into)
    }

    async fn limit_sell(&self, req: &OpenLimitOrderRequest) -> Result<Order<Self::OrderIdType>> {
        Coinbase::limit_sell(self, &req.symbol, req.size, req.price)
            .await
            .map(Into::into)
    }

    async fn market_buy(&self, req: &OpenMarketOrderRequest) -> Result<Order<Self::OrderIdType>> {
        Coinbase::market_buy(self, &req.symbol, req.size)
            .await
            .map(Into::into)
    }

    async fn market_sell(&self, req: &OpenMarketOrderRequest) -> Result<Order<Self::OrderIdType>> {
        Coinbase::market_sell(self, &req.symbol, req.size)
            .await
            .map(Into::into)
    }

    async fn cancel_order(
        &self,
        req: &CancelOrderRequest<Self::OrderIdType>,
    ) -> Result<OrderCanceled<Self::OrderIdType>> {
        Coinbase::cancel_order(self, req.id.clone(), req.pair.as_deref())
            .await
            .map(Into::into)
    }

    async fn cancel_all_orders(
        &self,
        req: &CancelAllOrdersRequest,
    ) -> Result<Vec<OrderCanceled<Self::OrderIdType>>> {
        Coinbase::cancel_all_orders(self, req.pair.as_deref())
            .await
            .map(|v| v.into_iter().map(Into::into).collect())
    }

    async fn get_all_open_orders(&self) -> Result<Vec<Order<Self::OrderIdType>>> {
        let params = GetOrderRequest {
            status: Some(String::from("open")),
            paginator: None,
            product_id: None,
        };

        Coinbase::get_orders(self, Some(&params))
            .await
            .map(|v| v.into_iter().map(Into::into).collect())
    }

    async fn get_order_history(
        &self,
        req: &GetOrderHistoryRequest,
    ) -> Result<Vec<Order<Self::OrderIdType>>> {
        let req: GetOrderRequest = req.into();

        Coinbase::get_orders(self, Some(&req))
            .await
            .map(|v| v.into_iter().map(Into::into).collect())
    }

    async fn get_account_balances(&self, paginator: Option<&Paginator>) -> Result<Vec<Balance>> {
        let paginator: Option<model::Paginator> = paginator.map(|p| p.into());

        Coinbase::get_account(self, paginator.as_ref())
            .await
            .map(|v| v.into_iter().map(Into::into).collect())
    }

    async fn get_trade_history(
        &self,
        req: &TradeHistoryRequest<Self::OrderIdType>,
    ) -> Result<Vec<Trade<Self::TradeIdType, Self::OrderIdType>>> {
        let req = req.into();

        Coinbase::get_fills(self, Some(&req))
            .await
            .map(|v| v.into_iter().map(Into::into).collect())
    }

    async fn get_price_ticker(&self, req: &GetPriceTickerRequest) -> Result<Ticker> {
        Coinbase::ticker(self, &req.symbol).await.map(Into::into)
    }

    async fn get_historic_rates(&self, req: &GetHistoricRatesRequest) -> Result<Vec<Candle>> {
        let params = CandleRequestParams::try_from(req)?;
        Coinbase::candles(self, &req.symbol, Some(&params))
            .await
            .map(|v| v.into_iter().map(Into::into).collect())
    }

    async fn refresh_market_info(&self) -> Result<()> {
        self.exchange_info.refresh(self).await
    }
}

impl From<model::Book<model::BookRecordL2>> for OrderBookResponse {
    fn from(book: model::Book<model::BookRecordL2>) -> Self {
        Self {
            last_update_id: None,
            bids: book.bids.into_iter().map(Into::into).collect(),
            asks: book.asks.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<model::BookRecordL2> for Bids {
    fn from(bids: model::BookRecordL2) -> Self {
        Self {
            price: bids.price,
            qty: bids.size,
        }
    }
}

impl From<model::BookRecordL2> for Asks {
    fn from(bids: model::BookRecordL2) -> Self {
        Self {
            price: bids.price,
            qty: bids.size,
        }
    }
}

impl From<model::Order> for Order<String> {
    fn from(order: model::Order) -> Self {
        Self {
            id: order.id,
            symbol: order.product_id,
            client_order_id: None,
            created_at: (order.created_at.timestamp_millis()) as u64,
        }
    }
}

impl From<String> for OrderCanceled<String> {
    fn from(id: String) -> Self {
        Self { id }
    }
}

impl From<model::Account> for Balance {
    fn from(account: model::Account) -> Self {
        Self {
            asset: account.currency,
            free: account.available,
            total: account.balance,
        }
    }
}

impl From<model::Fill> for Trade<u64, String> {
    fn from(fill: model::Fill) -> Self {
        Self {
            id: fill.trade_id,
            order_id: fill.order_id,
            pair: fill.product_id,
            price: fill.price,
            qty: fill.size,
            fees: fill.fee,
            side: match fill.side.as_str() {
                "buy" => Side::Buy,
                _ => Side::Sell,
            },
            liquidity: match fill.liquidity.as_str() {
                "M" => Some(Liquidity::Maker),
                "T" => Some(Liquidity::Taker),
                _ => None,
            },
            created_at: (fill.created_at.timestamp_millis()) as u64,
        }
    }
}

impl From<model::Ticker> for Ticker {
    fn from(ticker: model::Ticker) -> Self {
        Self {
            price: ticker.price,
        }
    }
}

impl From<model::Candle> for Candle {
    fn from(candle: model::Candle) -> Self {
        Self {
            time: candle.time * 1000,
            low: candle.low,
            high: candle.high,
            open: candle.open,
            close: candle.close,
            volume: candle.volume,
        }
    }
}

impl TryFrom<Interval> for u32 {
    type Error = OpenLimitError;
    fn try_from(value: Interval) -> Result<Self> {
        match value {
            Interval::OneMinute => Ok(60),
            Interval::FiveMinutes => Ok(300),
            Interval::FiftyMinutes => Ok(900),
            Interval::OneHour => Ok(3600),
            Interval::SixHours => Ok(21600),
            Interval::OneDay => Ok(86400),
            _ => Err(OpenLimitError::MissingParameter(format!(
                "{:?} is not supported in Coinbase",
                value,
            ))),
        }
    }
}

impl TryFrom<&GetHistoricRatesRequest> for model::CandleRequestParams {
    type Error = OpenLimitError;
    fn try_from(params: &GetHistoricRatesRequest) -> Result<Self> {
        let granularity = u32::try_from(params.interval)?;
        Ok(Self {
            daterange: params.paginator.clone().map(|p| p.into()),
            granularity: Some(granularity),
        })
    }
}

impl From<&GetOrderHistoryRequest> for model::GetOrderRequest {
    fn from(req: &GetOrderHistoryRequest) -> Self {
        Self {
            product_id: req.symbol.clone(),
            paginator: req.paginator.clone().map(|p| p.into()),
            status: None,
        }
    }
}

impl From<Paginator> for model::Paginator {
    fn from(paginator: Paginator) -> Self {
        Self {
            after: paginator.after,
            before: paginator.before,
            limit: paginator.limit,
        }
    }
}

impl From<&Paginator> for model::Paginator {
    fn from(paginator: &Paginator) -> Self {
        Self {
            after: paginator.after,
            before: paginator.before,
            limit: paginator.limit,
        }
    }
}

impl From<Paginator> for model::DateRange {
    fn from(paginator: Paginator) -> Self {
        Self {
            start: paginator.start_time.map(timestamp_to_datetime),
            end: paginator.end_time.map(timestamp_to_datetime),
        }
    }
}

impl From<&Paginator> for model::DateRange {
    fn from(paginator: &Paginator) -> Self {
        Self {
            start: paginator.start_time.map(timestamp_to_datetime),
            end: paginator.end_time.map(timestamp_to_datetime),
        }
    }
}

impl From<&TradeHistoryRequest<String>> for model::GetFillsReq {
    fn from(req: &TradeHistoryRequest<String>) -> Self {
        Self {
            order_id: req.order_id.clone(),
            paginator: req.paginator.clone().map(|p| p.into()),
            product_id: req.pair.clone(),
        }
    }
}
