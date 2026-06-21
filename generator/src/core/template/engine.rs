use handlebars::{Context, Handlebars, Helper, HelperResult, JsonRender, Output, RenderContext};
use serde_json::Value;

pub struct CoreTemplateEngine {
    reg: Handlebars<'static>,
}

impl CoreTemplateEngine {
    pub fn new() -> Self {
        let mut reg = Handlebars::new();
        reg.set_strict_mode(true);
        Self::register_helpers(&mut reg);
        Self { reg }
    }

    fn register_helpers(reg: &mut Handlebars) {
        reg.register_helper(
            "lookup", // 原来是 "array_get"
            Box::new(
                |h: &Helper,
                 _: &Handlebars,
                 _: &Context,
                 _rc: &mut RenderContext,
                 out: &mut dyn Output|
                 -> HelperResult {
                    let array_value = h.param(0).unwrap().value();
                    let index = h.param(1).unwrap().value().as_u64().unwrap() as usize;
                    if let Value::Array(arr) = array_value {
                        if let Some(elem) = arr.get(index) {
                            out.write(&elem.render())?;
                        } else {
                            out.write("")?;
                        }
                    }
                    Ok(())
                },
            ),
        );
    }

    /// Render a template string with the given data.
    pub fn render(
        &mut self,
        template_str: &str,
        data: &Value,
    ) -> Result<String, handlebars::RenderError> {
        // We need to register the template each time or use a template name.
        // For simplicity, we'll register with a fixed name.
        let template_name = "__core_template__";
        self.reg
            .register_template_string(template_name, template_str)?;
        self.reg.render(template_name, data)
    }
}

impl Default for CoreTemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}
