// fud, a macronutrient-based food record

use std::io;
use std::io::Write;

use clap::{App, SubCommand, Arg};

use chrono::prelude::*;

use sqlite::Value;

// TODO: These should be in a configuration file. They are not used yet.
// const FAT_FACTOR: f64 = 0.35;
// const CARBOHYDRATE_FACTOR: f64 = 1.25;
// const PROTEIN_FACTOR: f64 = 1.15;
// const WEIGHT_IN_KG: f64 = 86.0;

struct Food {
    food_code: String,
    description: String,
    portion_grams: f64,  // sqlite crate uses f64
    fat_grams: f64,
    carbohydrate_grams: f64,
    protein_grams: f64
}

struct Ingredient {
    date_stamp: String,
    meal_code: String,
    food_code: String,
    food_grams: f64
}

fn db() -> sqlite::Connection {
    let path = shellexpand::tilde("~/.fud/fud.db");
    sqlite::open(path.to_string()).expect("Could not open database")
}

fn calories_from_values(fat_grams: f64,
                        carbohydrate_grams: f64,
                        protein_grams: f64) -> f64 {
    fat_grams * 9.0 + carbohydrate_grams * 4.0 + protein_grams * 4.0
}

fn single_prompt(prompt_str: &str) -> String {
    print!("{}: ", prompt_str);
    if let Err(x) = io::stdout().flush() {
        println!("could not flush: {}", x);
    }

    let mut input = String::new();
    io::stdin().read_line(&mut input)
        .ok()
        .expect("Could not read line.");

    input.trim().to_string()
}

fn prompt(prompts: Vec<&str>) -> Vec<String> {
    let mut input_vec = Vec::new();
    for p in prompts {
        input_vec.push(single_prompt(p));
    }
    input_vec
}

fn check() {
    show_plan();
    let local: DateTime<Local> = Local::now();
    let date_stamp = local.format("%F");

    let connection = db();

    let statement = "
        SELECT m.meal_code,
            SUM(f.fat_grams / f.portion_grams * m.food_grams),
            SUM(f.carbohydrate_grams / f.portion_grams * m.food_grams),
            SUM(f.protein_grams / f.portion_grams * m.food_grams)
        FROM meals m, foods f
        WHERE m.food_code = f.food_code and m.datestamp=?
        GROUP BY
            (CASE m.meal_code
            WHEN 'B'
            THEN 1
            WHEN 'L'
            THEN 2
            WHEN 'A'
            THEN 3
            WHEN 'D'
            THEN 4
            WHEN 'E'
            THEN 5
            END)
    ";

    let mut cursor = connection
        .prepare(statement)
        .unwrap()
        .cursor();

    cursor.bind(&[Value::String(date_stamp.to_string())]).unwrap();

    println!("For {}", date_stamp.to_string());
    let mut fat_total: f64 = 0.0;
    let mut carbohydrate_total: f64 = 0.0;
    let mut protein_total: f64 = 0.0;
    let mut calorie_total: f64 = 0.0;

    println!("┌───┬───────┬───────┬───────┬────────┐");
    println!("│ M │   F   │   C   │   P   │  Cal   │");
    println!("├───┼───────┼───────┼───────┼────────┤");

    while let Some(row) = cursor.next().unwrap() {
        let meal_code = row[0].as_string().unwrap();
        let fat_grams = row[1].as_float().unwrap();
        let carbohydrate_grams = row[2].as_float().unwrap();
        let protein_grams = row[3].as_float().unwrap();
        let calories =
            calories_from_values(fat_grams, carbohydrate_grams, protein_grams);

        println!("│ {} │ {:5.1} │ {:5.1} │ {:5.1} │ {:6.0} │",
            meal_code,
            fat_grams,
            carbohydrate_grams,
            protein_grams,
            calories);

        fat_total += fat_grams;
        carbohydrate_total += carbohydrate_grams;
        protein_total += protein_grams;
        calorie_total += calories;
    }

    println!("├───┼───────┼───────┼───────┼────────┤");
    println!("│ T │ {:5.1} │ {:5.1} │ {:5.1} │ {:6.0} │",
            fat_total,
            carbohydrate_total,
            protein_total,
            calorie_total);

    println!("└───┴───────┴───────┴───────┴────────┘");
}

fn add_food(food: Food) {
    let statement = format!("insert into foods(food_code, description, portion_grams, fat_grams, carbohydrate_grams, protein_grams) values('{}','{}',{},{},{},{})", food.food_code, food.description, food.portion_grams, food.fat_grams, food.carbohydrate_grams, food.protein_grams);
    let connection = db();
    connection.execute(statement).unwrap();
}

fn prompt_food() {
    let prompts = vec![ "Food code"
                      , "Description"
                      , "Portion (g)"
                      , "Fat (g)"
                      , "Carbohydrates (g)"
                      , "Protein (g)"
                      ];

    let input_vec = prompt(prompts);

    let food_code = &input_vec[0];
    let description = &input_vec[1];
    let portion_grams_float = input_vec[2].parse::<f64>().expect("No parse");
    let fat_grams_float = input_vec[3].parse::<f64>().expect("No parse");
    let carbohydrate_grams_float = input_vec[4].parse::<f64>().expect("No parse");
    let protein_grams_float = input_vec[5].parse::<f64>().expect("No parse");

    add_food(Food {
        food_code: food_code.to_string(),
        description: description.to_string(),
        portion_grams: portion_grams_float,
        fat_grams: fat_grams_float,
        carbohydrate_grams: carbohydrate_grams_float,
        protein_grams: protein_grams_float
    });
}

fn add_ingredient(ingredient: Ingredient) {
    let statement = format!(
        "insert into meals(datestamp, meal_code, food_code, food_grams)
            values('{}','{}','{}',{})",
        ingredient.date_stamp,
        ingredient.meal_code,
        ingredient.food_code,
        ingredient.food_grams);
    let connection = db();
    connection.execute(statement).unwrap();
}

fn prompt_meal(iso_date: String) {
    // let local: DateTime<Local> = Local::now();
    // let iso_date = local.format("%F");
    println!("For {}", iso_date);
    let prompts = vec![ "Meal code"
                      , "Food code"
                      , "Portion size (g)"
                      ];

    let input_vec = prompt(prompts);

    let meal_code = &input_vec[0];
    let food_code = &input_vec[1];
    let food_grams = input_vec[2].parse::<f64>().expect("No parse");

    add_ingredient(Ingredient {
        date_stamp: iso_date.to_string(),
        meal_code: meal_code.to_string(),
        food_code: food_code.to_string(),
        food_grams: food_grams
    });

    let another = single_prompt("Another?");
    if another == "y" {
        prompt_meal(iso_date);
    }
}

fn list_foods() {
    let connection = db();

    let statement = "SELECT food_code,
                            description,
                            portion_grams,
                            fat_grams,
                            carbohydrate_grams,
                            protein_grams
                        FROM foods";

    let mut cursor = connection
                        .prepare(statement)
                        .unwrap()
                        .cursor();

    println!("| {:4} | {:38} | {:5} | {:5} | {:5} | {:5} |", "Food", "Description", "  g", "  F", "  C", "  P");
    while let Some(row) = cursor.next().unwrap() {
        println!("| {:4} | {:38} | {:5.1} | {:5.1} | {:5.1} | {:5.1} |",
        row[0].as_string().unwrap(),
        row[1].as_string().unwrap(),
        row[2].as_float().unwrap(),
        row[3].as_float().unwrap(),
        row[4].as_float().unwrap(),
        row[5].as_float().unwrap());
    }
}

fn list_meals() {
    println!("Meal list");
    let connection = db();

    let statement =
        "SELECT * FROM meals
            ORDER BY datestamp,
                (CASE meal_code
                    WHEN 'B'
                    THEN 1
                    WHEN 'L'
                    THEN 2
                    WHEN 'A'
                    THEN 3
                    WHEN 'D'
                    THEN 4
                    WHEN 'E'
                    THEN 5
                    END)
        ";

    let mut cursor = connection
                        .prepare(statement)
                        .unwrap()
                        .cursor();

    while let Some(row) = cursor.next().unwrap() {
        println!("| {} | {} | {} | {:5} |",
                row[0].as_string().unwrap(),
                row[1].as_string().unwrap(),
                row[2].as_string().unwrap(),
                row[3].as_float().unwrap());
    }
}

fn show_plan() {
    println!("Rough plan outline");
    println!("                          F    C   P");
    println!("Breakfast      7:00       10  40   40");
    println!("Lunch         11:00       20  70   80");
    println!("Afternoon     15:00        5  30   10");
    println!("Dinner        18:00       25  70   60");
    println!("Evening       22:00        5  30   30");
    println!("                          65 240  220");
}

fn main() {
    let app = App::new("fud")
                    .version("0.2.1")
                    .author("Will Langstroth <will@langstroth.com")
                    .about("Keep track of your eating.")
                    .subcommand(SubCommand::with_name("meal")
                        .about("Interactively add a meal")
                        .arg(Arg::with_name("date")
                            .short("d")
                            .long("date")
                            .value_name("DATE")
                            .help("Add a meal for another date")
                            .takes_value(true)))
                    .subcommand(SubCommand::with_name("meals")
                        .about("See recorded meals"))
                    .subcommand(SubCommand::with_name("food")
                        .about("Interactively add a food to the database"))
                    .subcommand(SubCommand::with_name("foods")
                        .about("See the list of foods"))
                    .subcommand(SubCommand::with_name("check")
                        .about("Check a day's meals (default today)")
                        .arg(Arg::with_name("date")
                            .short("d")
                            .long("date")
                            .value_name("DATE")
                            .help("Check meals from another date")
                            .takes_value(true)))
                    .subcommand(SubCommand::with_name("plan")
                        .about("See the meal plan"));

    let matches = app.get_matches();

    let date: DateTime<Local> = Local::now();
    let mut iso_date: String = date.format("%F").to_string();

    // This looks rough. Is clap really like this?
    if let Some(s) = matches.subcommand_matches("meal") {
        if let Some(d) = s.value_of("date") {
            println!("{}", date);
            iso_date = d.to_string();
        }
    }

    println!("{:?}", matches);

    match matches.subcommand_name() {
        Some("food") => prompt_food(),
        Some("meal") => prompt_meal(iso_date),
        Some("foods") => list_foods(),
        Some("meals") => list_meals(),
        Some("plan") => show_plan(),
        Some("check") => check(),
        None => (),
        _ => println!("Some other subcommand was used"),
    }
}
