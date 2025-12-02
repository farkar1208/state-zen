//! State-Zen 状态机框架示例程序
//! 
//! 演示如何使用状态机框架

use STATE_ZEN::examples::player_movement;

fn main() {
    println!("State-Zen 状态机框架示例");
    println!("========================\n");
    
    // 运行玩家移动示例
    player_movement::run_player_movement_example();
    
    println!("所有示例运行完成！");
}