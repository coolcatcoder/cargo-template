#![feature(try_trait_v2, try_trait_v2_residual)]
use std::{
    env::args,
    fmt::Debug,
    ops::{ControlFlow, FromResidual, Residual, Try},
    panic::Location,
    path::{Path, PathBuf},
    process::{Command, ExitCode, Termination},
};

#[expect(clippy::box_collection)]
#[derive(Clone, Debug, Hash, PartialEq, PartialOrd, Eq, Ord)]
struct MaybeError(Box<Vec<(String, &'static Location<'static>)>>);
impl MaybeError {
    #[track_caller]
    fn new(display: String) -> Self {
        Self(Box::new(vec![(display, Location::caller())]))
    }
}

#[derive(Clone, Debug, Hash, PartialEq, PartialOrd, Eq, Ord)]
#[must_use = "this `Maybe` may be an `Err` variant, which should be handled"]
enum Maybe<T> {
    Ok(T),
    Err(MaybeError),
}

impl<T> Try for Maybe<T> {
    type Output = T;
    type Residual = Maybe<!>;

    fn from_output(output: Self::Output) -> Self {
        Maybe::Ok(output)
    }

    fn branch(self) -> std::ops::ControlFlow<Self::Residual, Self::Output> {
        match self {
            Maybe::Ok(output) => ControlFlow::Continue(output),
            Maybe::Err(residual) => ControlFlow::Break(Maybe::Err(residual)),
        }
    }
}
impl<T> Residual<T> for Maybe<!> {
    type TryType = Maybe<T>;
}
impl<T> FromResidual for Maybe<T> {
    #[track_caller]
    fn from_residual(residual: <Self as Try>::Residual) -> Self {
        let Maybe::Err(mut residual) = residual;
        residual.0.push((String::new(), Location::caller()));
        Maybe::Err(residual)
    }
}
impl<T> FromResidual<Option<!>> for Maybe<T> {
    #[track_caller]
    fn from_residual(_residual: Option<std::convert::Infallible>) -> Self {
        Self::Err(MaybeError::new("Option was None.".into()))
    }
}
impl<T, E: Debug> FromResidual<Result<!, E>> for Maybe<T> {
    #[track_caller]
    fn from_residual(residual: Result<!, E>) -> Self {
        let Err(residual) = residual;
        Self::Err(MaybeError::new(format!("{residual:?}")))
    }
}

impl<T: Termination> Termination for Maybe<T> {
    fn report(self) -> ExitCode {
        match self {
            Maybe::Ok(output) => output.report(),
            Maybe::Err(residual) => {
                eprintln!("Error:");
                for (display, location) in residual.0.iter() {
                    eprintln!("{location} {display}");
                }

                ExitCode::FAILURE
            }
        }
    }
}

fn main() -> Maybe<()> {
    let mut args = args().peekable();

    args.next()?;
    args.next()?;

    let template = args.next()?;
    let template_root = [
        "/home/coolcatcoder/Documents/GitHub/cargo-template/assets/",
        &template,
    ]
    .into_iter()
    .collect::<PathBuf>();

    let project_name = args.peek()?;
    let project_root: PathBuf = project_name.into();

    Command::new("cargo").arg("new").args(args).output()?;

    recursive_apply_template(&template_root, &project_root)?;

    Maybe::Ok(())
}

fn recursive_apply_template(template_root: &Path, project_root: &Path) -> Maybe<()> {
    for entry in std::fs::read_dir(template_root)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        let file_name = entry.file_name();
        let project_path = project_root.join(&file_name);
        let template_path = template_root.join(file_name);

        match metadata.is_dir() {
            true => {
                std::fs::create_dir(&project_path)?;
                recursive_apply_template(&template_path, &project_path)?;
            }
            false => {
                let mut file = std::fs::OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(project_path)?;

                let mut content = std::fs::OpenOptions::new().read(true).open(template_path)?;

                std::io::copy(&mut content, &mut file)?;
            }
        }
    }

    Maybe::Ok(())
}
