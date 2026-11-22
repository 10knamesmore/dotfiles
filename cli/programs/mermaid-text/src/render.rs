use crate::{
    Options, charts, color,
    graphics::{ImageSize, Scene},
    source::Source,
};

pub(crate) struct Diagram {
    source: Source,
    chart: &'static charts::Entry,
    options: Options,
}

impl Diagram {
    pub fn parse(input: &str, options: &Options) -> Result<Self, String> {
        Self::new(Source::parse(input)?, options)
    }

    pub fn new(mut source: Source, options: &Options) -> Result<Self, String> {
        let chart = charts::lookup(&source.header)?;
        let options = Options {
            config: std::mem::take(&mut source.config),
            ..*options
        };
        Ok(Self {
            source,
            chart,
            options,
        })
    }

    pub fn png(self, size: ImageSize) -> Result<Vec<u8>, String> {
        let draw = self.chart.image_renderer()?;
        let mut scene = Scene::new(&self.options)?;
        scene.set_title_color(self.chart.title_color(&self.options));
        draw(&self.source.body, &self.options, &mut scene)
            .map_err(|err| self.source.diagnostic(err))?;
        scene.png(
            self.source.title.as_deref(),
            self.source.description.as_deref(),
            size,
        )
    }

    pub fn text(self) -> Result<String, String> {
        let Self {
            source,
            chart,
            options,
        } = self;
        let diagram = (chart.text)(&source.body, &options).map_err(|err| source.diagnostic(err))?;
        let mut output = match source.title {
            Some(mut title) => {
                if let Some(fg) = chart.title_color(&options) {
                    title = color::with_base(
                        &title,
                        color::Paint {
                            fg: Some(fg),
                            bg: None,
                        },
                    )?;
                }
                format!("{title}\n\n{diagram}")
            }
            None => diagram,
        };
        if let Some(description) = source.description {
            output.push_str("\n\n");
            output.push_str(&description);
        }
        if output.chars().any(|ch| ch.is_control() && ch != '\n') {
            if !options.color {
                return Err("diagram contains a non-printable character".into());
            }
            for line in output.split('\n') {
                color::validate_line(line)?;
            }
        }
        if options.color {
            let base = color::Paint {
                fg: options.config.theme_color("textColor"),
                bg: options.config.theme_color("background"),
            };
            if base != color::Paint::default() {
                return color::with_base(&output, base);
            }
        }
        Ok(output)
    }
}
