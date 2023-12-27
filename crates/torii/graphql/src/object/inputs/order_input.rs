use async_graphql::dynamic::{Enum, Field, InputObject, InputValue, ResolverContext, TypeRef};

use super::InputObjectTrait;
use crate::object::TypeMapping;
use crate::query::order::{Direction, Order};

pub struct OrderInputObject {
    pub type_name: String,
    pub type_mapping: TypeMapping,
}

impl OrderInputObject {
    pub fn new(type_name: &str, object_types: &TypeMapping) -> Self {
        println!("OrderInputObject {}", type_name);
        Self { type_name: format!("{}Order", type_name), type_mapping: object_types.clone() }
    }
}

impl InputObjectTrait for OrderInputObject {
    fn type_name(&self) -> &str {
        &self.type_name
    }

    fn type_mapping(&self) -> &TypeMapping {
        &self.type_mapping
    }

    fn input_object(&self) -> InputObject {
        // direction and field values are required (not null)
        let mut input_object =  InputObject::new(self.type_name())
            .field(InputValue::new(
                "field",
                TypeRef::named_nn(format!("{}Field", self.type_name())),
            ));
        if &self.type_name != "World__ModelOrder" {
            input_object = input_object.field(InputValue::new(
                "direction",
                TypeRef::named_nn("OrderDirection"),
            ));
        }
        input_object
    }

    fn enum_objects(&self) -> Option<Vec<Enum>> {
        // empty vec
        let mut vec = Vec::new();

        // Field Order enum consist of all members of a model
        let field_order = self
            .type_mapping
            .iter()
            .fold(Enum::new(format!("{}Field", self.type_name())), |acc, (ty_name, _)| {
                acc.item(ty_name.to_uppercase())
            });
        vec.push(field_order);

        if &self.type_name != "World__ModelOrder" {
            // Direction enum has only two members ASC and DESC
            let direction = Enum::new("OrderDirection").item("ASC").item("DESC");
            vec.push(direction);
        }

        Some(vec)
    }
}

pub fn order_argument(field: Field, type_name: &str) -> Field {
    println!("order_argument {}", type_name);
    field.argument(InputValue::new("order", TypeRef::named(format!("{}Order", type_name))))
}

pub fn parse_order_argument(ctx: &ResolverContext<'_>) -> Option<Order> {
    let order_input = ctx.args.get("order")?;
    let input_object = order_input.object().ok()?;
    let dir_value = input_object.get("direction")?;
    let field_value = input_object.get("field")?;

    let direction = Direction::try_from(dir_value.enum_name().ok()?).ok()?;
    let field = field_value.enum_name().ok()?.to_lowercase();
    Some(Order { direction, field })
}
