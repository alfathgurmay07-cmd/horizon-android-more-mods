// Adapted from Horizon game.rs at 81ae4a7d8f27d38e5cabbdcda0a06b2c4b000045 (MIT).
use super::*;
pub(super) fn render(game: &mut crate::game::Game<'_>, alpha: f64) -> Vec<Vertex> {
    let mut out = Vec::new();
    // GameRenderer.renderItemInHand: F1 hides the hand with the HUD.
    if game.spectator()
        || game.ui.hide_gui
        || game.sleeping.is_some()
        || !game.camera_type.first_person()
        || game
            .player
            .as_ref()
            .is_none_or(|p| p.health.dead_or_dying())
        || game.screen == Screen::Dead
    {
        return out;
    }
    let a = alpha as f32;
    // FirstPersonHandsAndItemsRenderer: the swing goes to the hand that
    // swings; each hand has its inverse arm height (the swap scale is 1
    // for every item but spears).
    let swing = game.swing.animation(a);
    let attacks = Hand::BOTH.map(|h| {
        if game.swing.attack_hand() == h {
            swing
        } else {
            0.0
        }
    });
    let heights = game.heights.interpolated(a);
    // The hand sway: both hands turn by a tenth of the bobs' lag.
    let to_world = game.sway.to_world(a, &game.view);
    let eye = game.eye_position().unwrap_or_default();
    let light = game.world.light(
        eye[0].floor() as i32,
        eye[1].floor() as i32,
        eye[2].floor() as i32,
    );
    let packed = (u32::from(light >> 4) * 16) | ((u32::from(light & 15) * 16) << 8);
    let shown = game.hands_to_render();
    let used_hand = game.used_hand();
    let spinning = game
        .player
        .as_ref()
        .is_some_and(|p| p.auto_spin_attack_ticks > 0);
    let items = punchy::rendered_items(game);
    let off_hand_empty = items[1].is_none();
    for (i, (hand, item)) in Hand::BOTH.into_iter().zip(items).enumerate() {
        if !shown[i] {
            continue;
        }
        let scale = item.as_ref().map_or(1.0, |stack| {
            game.pack
                .item_renderer
                .models
                .swap_animation_scale(&horizon_client::item_render::stack_view(stack))
        });
        let attack = attacks[i];
        let inverse_height = horizon_client::first_person::equip_displacement(heights[i], scale);
        let mut canceled = false;
        game.native_render_hand(
            hand,
            item.as_ref(),
            a,
            attack,
            inverse_height,
            &mut canceled,
        );
        if canceled {
            continue;
        }
        // The main hand on its arm, the off hand on the other one.
        let left = game.settings.main_hand_left != (hand == Hand::Off);
        let Some(stack) = item else {
            // renderArmWithItem: only the main hand's empty arm shows,
            // on the main arm (the left one with the main hand set to
            // left), and not while the player is invisible.
            if hand == Hand::Off || game.own_invisible() {
                continue;
            }
            // renderPlayerArm
            let m = to_world * crate::game::punchy::arm_pose(left, inverse_height, attack);
            game.first_person_arm(left, m, packed, &mut out);
            continue;
        };
        if let Some(id) = maps::map_id(&stack) {
            use horizon_client::first_person::{MapHold, map_arm_poses, one_handed_map_pose};
            let hold = MapHold::of(hand, off_hand_empty);
            // renderOneHandedMap and renderTwoHandedMap: the arms under
            // the map, unless the player is invisible.
            if !game.own_invisible() {
                for (arm, pose) in map_arm_poses(hold, left, game.pitch, inverse_height, attack) {
                    game.first_person_arm(arm, to_world * pose, packed, &mut out);
                }
            }
            match hold {
                MapHold::TwoHanded => game.two_handed_map_geometry(
                    to_world,
                    attack,
                    inverse_height,
                    id,
                    packed,
                    &mut out,
                ),
                MapHold::OneHanded => game.held_map(
                    to_world * one_handed_map_pose(left, inverse_height, attack),
                    id,
                    packed,
                    &mut out,
                ),
            }
            continue;
        }
        let mut owner = game.player_owner(Some(&stack));
        // The other hand may hold an equal stack: only the used
        // hand's item is in use, only the main hand's selected.
        owner.using_this_item &= used_hand == Some(hand);
        if hand == Hand::Off {
            owner.is_selected = false;
            owner.is_carried = false;
        }
        // The hand renderer's copy of the main hand stack.
        owner.fishing_cast_with_this_item = game.main_hand_rod() && game.cast_rod() == Some(&stack);
        let using = owner.using_this_item && owner.use_item_remaining_ticks > 0;
        let ticks_used =
            owner.use_duration as f32 - (owner.use_item_remaining_ticks as f32 - a + 1.0);
        use horizon_client::first_person::ItemPose;
        // ItemUseAnimation.SPEAR.
        let spear = using
            .then(|| horizon_client::spear::Kinetic::of(&stack.item))
            .flatten()
            .map(|kinetic| ItemPose::Spear {
                kinetic,
                time_held: ticks_used,
                ticks_since_hit: game.kinetic_hit_ticks(a),
            });
        let pose = match &*stack.item {
            _ if let Some(pose) = spear => pose,
            "minecraft:bow" if using => ItemPose::Bow { ticks_used },
            "minecraft:crossbow" => {
                let charged = stack.data().is_some_and(|d| !d.charged.is_empty());
                ItemPose::Crossbow {
                    ticks_used: (using && !charged).then_some(ticks_used),
                    charge_duration: owner.crossbow_charge_duration as f32,
                    // Only the main hand turns a charged crossbow aside.
                    charged: charged && hand == Hand::Main,
                }
            }
            "minecraft:trident" if using => ItemPose::Trident { ticks_used },
            // ItemUseAnimation.BRUSH: applyBrushTransform.
            "minecraft:brush" if using => ItemPose::Brush {
                remaining: owner.use_item_remaining_ticks as f32,
                partial: a,
            },
            _ if using => {
                let components = game
                    .pack
                    .item_renderer
                    .models
                    .components(&horizon_client::item_render::stack_view(&stack));
                let mut animation = components
                    .get("minecraft:consumable")
                    .map_or(horizon_world::consumable::UseAnimation::None, |c| {
                        horizon_client::first_person::consumable_animation(c)
                    });
                game.native_use_animation(&stack, &mut animation);
                item_use::use_pose(
                    animation,
                    owner.use_item_remaining_ticks as f32,
                    owner.use_duration as f32,
                    a,
                )
            }
            _ if spinning => ItemPose::SpinAttack,
            // ItemInHandRenderer: a STAB swing of this hand jabs.
            _ if game
                .swing
                .current
                .is_some_and(|c| c.hand == hand && c.animation.kind == 2) =>
            {
                ItemPose::Stab
            }
            _ => ItemPose::Swing,
        };
        let m = to_world * crate::game::punchy::item_pose(left, inverse_height, attack, pose);
        // Seeded by the player id plus the context ordinal.
        let (context, seed) = if left {
            (
                if crate::game::punchy::authored(left) {
                    horizon_assets::item_model::DisplayContext::ThirdPersonLeftHand
                } else {
                    horizon_assets::item_model::DisplayContext::FirstPersonLeftHand
                },
                3,
            )
        } else {
            (
                if crate::game::punchy::authored(left) {
                    horizon_assets::item_model::DisplayContext::ThirdPersonRightHand
                } else {
                    horizon_assets::item_model::DisplayContext::FirstPersonRightHand
                },
                4,
            )
        };
        // Punchy sets neutralize-use while resolving its vanilla item model:
        // authored clips already supply the use pose (IsUsingItemProperty and
        // UseDurationProperty mixins), so do not apply a second vanilla pose.
        if crate::game::punchy::authored(left) {
            owner.using_this_item = false;
            owner.use_item_remaining_ticks = 0;
        }
        let state = game.resolve_item(&stack, context, Some(&owner), seed);
        let light = game.world.light(
            eye[0].floor() as i32,
            eye[1].floor() as i32,
            eye[2].floor() as i32,
        );
        game.submit_item_world(
            &state,
            m,
            horizon_client::block_entity_render::vertex_light(light),
            &mut out,
        );
    }
    // The hand has its own submit storage, which never uses improved
    // transparency (`LevelRenderer` alone calls `setUseImprovedTransparency`):
    // a held player head stays classic.
    for vertex in &mut out {
        vertex.layer &= !horizon_client::gfx::OIT_ENTITY;
    }
    out
}
