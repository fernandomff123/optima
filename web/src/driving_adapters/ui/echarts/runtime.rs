#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(inline_js = r#"
function observeOptimaEChart(host) {
  if (!globalThis.ResizeObserver || host.__optimaResizeObserver) return;
  const observer = new ResizeObserver(() => {
    const instance = globalThis.echarts?.getInstanceByDom(host);
    if (instance) instance.resize();
  });
  observer.observe(host);
  host.__optimaResizeObserver = observer;
}

export function renderOptimaEChart(hostId, optionJson) {
  const host = document.getElementById(hostId);
  if (!host || !globalThis.echarts) return false;
  const chart = globalThis.echarts.getInstanceByDom(host)
    || globalThis.echarts.init(host, null, { renderer: 'canvas' });
  observeOptimaEChart(host);
  chart.setOption(JSON.parse(optionJson), { notMerge: true, lazyUpdate: false });
  chart.resize();
  return true;
}

export function resizeOptimaEChart(hostId) {
  const host = document.getElementById(hostId);
  if (!host || !globalThis.echarts) return false;
  const chart = globalThis.echarts.getInstanceByDom(host);
  if (!chart) return false;
  chart.resize();
  return true;
}

export function disposeOptimaEChart(hostId) {
  const host = document.getElementById(hostId);
  if (!host || !globalThis.echarts) return;
  const chart = globalThis.echarts.getInstanceByDom(host);
  if (host.__optimaResizeObserver) {
    host.__optimaResizeObserver.disconnect();
    delete host.__optimaResizeObserver;
  }
  if (chart) chart.dispose();
}
"#)]
extern "C" {
    #[wasm_bindgen::prelude::wasm_bindgen(js_name = renderOptimaEChart)]
    fn render_echart(host_id: &str, option_json: &str) -> bool;

    #[wasm_bindgen::prelude::wasm_bindgen(js_name = resizeOptimaEChart)]
    fn resize_echart(host_id: &str) -> bool;

    #[wasm_bindgen::prelude::wasm_bindgen(js_name = disposeOptimaEChart)]
    fn dispose_echart(host_id: &str);
}

#[cfg(not(target_arch = "wasm32"))]
fn render_echart(_host_id: &str, _option_json: &str) -> bool {
    false
}

#[cfg(not(target_arch = "wasm32"))]
fn resize_echart(_host_id: &str) -> bool {
    false
}

#[cfg(not(target_arch = "wasm32"))]
fn dispose_echart(_host_id: &str) {}

pub fn render_chart(host_id: &str, option_json: &str) -> bool {
    render_echart(host_id, option_json)
}

pub fn resize_chart(host_id: &str) -> bool {
    resize_echart(host_id)
}

pub fn dispose_chart(host_id: &str) {
    dispose_echart(host_id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_runtime_is_a_safe_noop_for_unit_tests() {
        assert!(!render_chart("chart", "{}"));
        assert!(!resize_chart("chart"));
        dispose_chart("chart");
    }
}
