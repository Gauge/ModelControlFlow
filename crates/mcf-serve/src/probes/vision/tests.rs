#![allow(clippy::panic, clippy::expect_used)]

use super::*;
use crate::probes::tests::plain;

#[test]
fn the_two_pictures_are_not_the_same_picture() {
    let triangle = image(Shape::Triangle);
    let circle = image(Shape::Circle);
    assert_ne!(triangle, circle, "two pictures that differ is the method");
    assert!(!triangle.is_empty() && !circle.is_empty());
}

#[test]
fn a_picture_is_the_same_picture_every_time() {
    assert_eq!(image(Shape::Triangle), image(Shape::Triangle));
    assert_eq!(image(Shape::Circle), image(Shape::Circle));
}

#[test]
fn each_shape_is_drawn_where_it_is_said_to_be() {
    assert!(in_triangle(224, 300), "the triangle covers its own middle");
    assert!(in_circle(224, 224), "the circle covers its own centre");
    for (x, y) in [(0_i64, 0_i64), (447, 0), (0, 447), (447, 447)] {
        assert!(!in_triangle(x, y), "the triangle does not reach {x},{y}");
        assert!(!in_circle(x, y), "the circle does not reach {x},{y}");
    }
    assert!(in_triangle(224, 100));
    assert!(!in_circle(224, 100));
}

#[test]
fn no_projector_is_inconclusive_rather_than_a_refusal() {
    let mut never = |_: &[u8], _: &str| None;
    let probed = vision(
        std::path::Path::new("/nowhere/a-model.gguf"),
        b"not a model",
        None,
        "a test",
        &mut never,
    );
    assert!(
        matches!(probed.outcome, Outcome::Inconclusive { .. }),
        "a file that is not a model is inconclusive, never a negative result"
    );
}

#[test]
fn two_different_answers_are_what_vision_looks_like() {
    let mut said = ["a red triangle".to_owned(), "a blue circle".to_owned()].into_iter();
    let mut look = |_: &[u8], _: &str| said.next().map(|held| (held, 8));
    let probed = vision(
        std::path::Path::new("/nowhere/a-model.gguf"),
        &plain(),
        Some(std::path::Path::new("/nowhere/mmproj.gguf")),
        "a test",
        &mut look,
    );
    match probed.outcome {
        Outcome::Observed(sees) => {
            assert!(
                sees.answers_differ,
                "different answers means the image got in"
            );
            assert_eq!(probed.trials, 2);
            assert_eq!(probed.tokens, 16, "what both turns spent, added");
        }
        other @ Outcome::Inconclusive { .. } => {
            panic!("expected an observation, got {other:?}")
        }
    }
}

#[test]
fn one_answer_for_two_pictures_is_no_vision() {
    let mut look = |_: &[u8], _: &str| Some(("I cannot see an image.".to_owned(), 5));
    let probed = vision(
        std::path::Path::new("/nowhere/a-model.gguf"),
        &plain(),
        Some(std::path::Path::new("/nowhere/mmproj.gguf")),
        "a test",
        &mut look,
    );
    match probed.outcome {
        Outcome::Observed(sees) => assert!(
            !sees.answers_differ,
            "the same answer twice means the pixels did not reach the model"
        ),
        other @ Outcome::Inconclusive { .. } => {
            panic!("expected an observation, got {other:?}")
        }
    }
}

#[test]
fn an_answer_differing_only_in_spacing_is_the_same_answer() {
    let mut said = ["the same\n".to_owned(), "  the same  ".to_owned()].into_iter();
    let mut look = |_: &[u8], _: &str| said.next().map(|held| (held, 1));
    let probed = vision(
        std::path::Path::new("/nowhere/a-model.gguf"),
        &plain(),
        Some(std::path::Path::new("/nowhere/mmproj.gguf")),
        "a test",
        &mut look,
    );
    match probed.outcome {
        Outcome::Observed(sees) => assert!(
            !sees.answers_differ,
            "trailing space is not the model seeing something"
        ),
        other @ Outcome::Inconclusive { .. } => {
            panic!("expected an observation, got {other:?}")
        }
    }
}
