use crate::application::asset_chart::Candle;
use leptos::prelude::*;
use wasm_bindgen::prelude::*;

const HOST_ID: &str = "asset-candlestick-chart";

#[wasm_bindgen(inline_js = r#"
export function renderAssetChart(id, payload) {
  const host = document.getElementById(id);
  if (!host || !globalThis.echarts) return;
  disposeAssetChart(id);
  const input = JSON.parse(payload);
  const chart = globalThis.echarts.init(host, null, {renderer:'canvas'});
  const up='#35A95C', down='#DE3436', muted='#78828D', grid='#212935';
  const volumes=input.candles.map((c,i)=>({value:c[4],itemStyle:{color:c[1]>=c[0]?up:down}}));
  chart.setOption({animation:false,backgroundColor:'transparent',axisPointer:{link:[{xAxisIndex:'all'}]},tooltip:{trigger:'axis',axisPointer:{type:'cross'},backgroundColor:'#0E1A27',borderColor:'#1C2530',textStyle:{color:'#FFFFFF'}},grid:[{left:52,right:18,top:24,height:'67%'},{left:52,right:18,top:'76%',height:'16%'}],xAxis:[{type:'category',data:input.dates,boundaryGap:true,axisLine:{lineStyle:{color:'#1C2530'}},axisLabel:{color:muted,interval:3},splitLine:{show:false}},{type:'category',gridIndex:1,data:input.dates,boundaryGap:true,axisLabel:{show:false},axisLine:{lineStyle:{color:'#1C2530'}},axisTick:{show:false}}],yAxis:[{scale:true,position:'right',splitNumber:6,axisLabel:{color:muted,formatter:v=>'$'+v.toFixed(0)},splitLine:{lineStyle:{color:grid,type:'dashed'}}},{scale:true,gridIndex:1,position:'right',axisLabel:{color:muted,formatter:v=>(v/1000000).toFixed(0)+'M'},splitLine:{show:false}}],dataZoom:[{type:'inside',xAxisIndex:[0,1],start:0,end:100}],series:[{name:'AAPL',type:'candlestick',data:input.candles.map(c=>c.slice(0,4)),itemStyle:{color:up,color0:down,borderColor:up,borderColor0:down}},{name:'Volume',type:'bar',xAxisIndex:1,yAxisIndex:1,data:volumes,barMaxWidth:12}]});
  const observer = new ResizeObserver(()=>chart.resize()); observer.observe(host);
  host.__optimaChart={chart,observer};
}
export function disposeAssetChart(id) {
  const host=document.getElementById(id); const state=host && host.__optimaChart;
  if(state){state.observer.disconnect();state.chart.dispose();delete host.__optimaChart;}
}
"#)]
extern "C" {
    #[wasm_bindgen(js_name = renderAssetChart)]
    fn render(id: &str, payload: &str);
    #[wasm_bindgen(js_name = disposeAssetChart)]
    fn dispose(id: &str);
}

#[component]
pub fn AssetCandlestickChart(candles: Vec<Candle>) -> impl IntoView {
    let payload = payload(&candles);
    Effect::new(move |_| render(HOST_ID, &payload));
    on_cleanup(move || dispose(HOST_ID));
    view! { <div id=HOST_ID class="h-full min-h-[28rem] w-full" role="img" aria-label="Deterministic AAPL daily candlestick and volume chart"></div> }
}

fn payload(candles: &[Candle]) -> String {
    let dates = candles
        .iter()
        .map(|c| format!("\"{}\"", c.date))
        .collect::<Vec<_>>()
        .join(",");
    let values = candles
        .iter()
        .map(|c| {
            format!(
                "[{:.2},{:.2},{:.2},{:.2},{}]",
                c.open, c.close, c.low, c.high, c.volume
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(r#"{{"dates":[{dates}],"candles":[{values}]}}"#)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn adapter_serializes_echarts_open_close_low_high_order() {
        let json = payload(&[Candle {
            date: "May 01".into(),
            open: 188.0,
            close: 191.0,
            low: 187.0,
            high: 192.0,
            volume: 42,
        }]);
        assert_eq!(
            json,
            r#"{"dates":["May 01"],"candles":[[188.00,191.00,187.00,192.00,42]]}"#
        );
    }
}
