#[cfg(test)]
mod tests {
    use crate::b_yawl::engine::BYawlEngine;
    use crate::b_yawl::patterns::BYawlPatternCompiler;

    #[test]
    fn case_study_01_simple_make_trip_process() {
        // Based on YAWL User Manual 5.1 - Section 5.5.2 Example 2: Simple Make Trip Process
        // The user registers for a trip, which triggers an OR-split (WCP-06) allowing them
        // to selectively book a flight, book a hotel, and/or book a car.
        // Finally, an OR-join (WCP-07) synchronizes whatever bookings were chosen, leading to payment.

        let mut engine = BYawlEngine::new();
        engine.state_mask = 0b1; // Start token at p0 (Register Ready)

        // Task 1: Register (OR-Split)
        // Consumes p0 (0b1), produces to p1 (flight), p2 (hotel), p3 (car)
        let t_register = BYawlPatternCompiler::wcp06_multi_choice(1, 0b1, 0b1110);

        // Let's assume the user only chooses to book a flight and a hotel (not a car)
        // In bYAWL, dynamic OR-split routing is managed by the environment setting the precise produce_mask
        // For this test, we override the produce_mask at runtime to simulate the user's choice:
        let mut t_register_runtime = t_register;
        t_register_runtime.produce_mask = 0b0110; // Only flight (p1) and hotel (p2)

        let executed = engine.execute_task(&t_register_runtime);
        assert!(executed, "Register task must execute");
        assert_eq!(engine.state_mask, 0b0110, "Tokens should be at p1 and p2");

        // Tasks 2, 3, 4: Book Flight, Book Hotel, Book Car
        let t_book_flight = BYawlPatternCompiler::wcp01_sequence(2, 0b0010, 0b10000); // p1 -> p4
        let t_book_hotel = BYawlPatternCompiler::wcp01_sequence(3, 0b0100, 0b100000); // p2 -> p5
        let t_book_car = BYawlPatternCompiler::wcp01_sequence(4, 0b1000, 0b1000000); // p3 -> p6

        // Execute Flight
        let executed_flight = engine.execute_task(&t_book_flight);
        assert!(executed_flight, "Flight booking must execute");
        assert_eq!(engine.state_mask, 0b10100, "Tokens should be at p4 and p2");

        // Attempt Car (Should Fail)
        let executed_car = engine.execute_task(&t_book_car);
        assert!(
            !executed_car,
            "Car booking must not execute as it wasn't selected"
        );

        // Task 5: Pay (OR-Join)
        // Consumes p4, p5, p6 and synchronizes them into p7
        // Reachability mask: Any token at p1, p2, p3, p4, p5, p6 can reach the join.
        let reachability_mask = 0b1111110;
        let t_pay = BYawlPatternCompiler::wcp07_structured_synchronizing_merge(
            5,
            0b1110000,
            0b10000000,
            reachability_mask,
        );

        // Attempt Pay (Should Fail because Hotel is still pending)
        let executed_pay_early = engine.execute_task(&t_pay);
        assert!(
            !executed_pay_early,
            "Pay must synchronize and wait for all selected bookings to finish"
        );

        // Execute Hotel
        let executed_hotel = engine.execute_task(&t_book_hotel);
        assert!(executed_hotel, "Hotel booking must execute");
        assert_eq!(engine.state_mask, 0b110000, "Tokens should be at p4 and p5");

        // Attempt Pay again
        let executed_pay = engine.execute_task(&t_pay);
        assert!(
            executed_pay,
            "Pay must execute now that flight and hotel are complete, and car was never started"
        );
        assert_eq!(
            engine.state_mask, 0b10000000,
            "Process completes successfully at p7"
        );
    }

    #[test]
    fn case_study_02_make_trip_process_multiple_instances() {
        // Based on YAWL User Manual 5.1 - Section 5.5.3 Example 3: Make Trip Process with Multiple Instance Composite Tasks
        // The trip has several legs. A multiple instance task "do itinerary segment" is spawned for each leg.

        let mut engine = BYawlEngine::new();
        engine.state_mask = 0b1; // Start token at p0

        // In bYAWL, we use WCP-14 Multiple Instances with Priori Run-Time Knowledge.
        // Assume the user has a trip with 3 legs (e.g., flight to London, train to Paris, flight home).
        let t_spawn_legs = BYawlPatternCompiler::wcp14_mi_priori_runtime(1, 0b1, 0b10, 3);

        let executed = engine.execute_task(&t_spawn_legs);
        assert!(executed);

        // p1 now contains the active state, and the active_instances array tracks the 3 spawned legs.
        assert_eq!(engine.state_mask, 0b10);
        assert_eq!(
            engine.active_instances[1], 3,
            "3 itinerary segments spawned"
        );

        // The composite sub-net tasks would process each leg...
        // We simulate completing the MI activity using WCP-27 (Complete MI Activity).
        // This task fires to wrap up the instances and move to the final billing stage.
        let t_complete_legs =
            BYawlPatternCompiler::wcp27_complete_mi_activity(2, 0b10, 0b100, 0b10);

        let executed_completion = engine.execute_task(&t_complete_legs);
        assert!(executed_completion);

        // The active instances array should be reset, and the process moves to p2 (Calculate Total Payment).
        assert_eq!(engine.state_mask, 0b100);
        assert_eq!(
            engine.active_instances[1], 0,
            "All itinerary segments completed and reset"
        );
    }

    #[test]
    fn case_study_03_credit_rating_process() {
        // Based on YAWL User Manual 5.1 - Section 5.5.1 Example 1: Credit Rating Process
        // Simple sequential flow with an Exclusive Choice (WCP-04) and Simple Merge (WCP-05).

        let mut engine = BYawlEngine::new();
        engine.state_mask = 0b1; // p0: Receive Application

        // Task 1: Receive Application
        let t_receive = BYawlPatternCompiler::wcp01_sequence(1, 0b1, 0b10);
        engine.execute_task(&t_receive);

        // Task 2: Check Credit (XOR Split)
        // Evaluates SSN and routes to either p2 (Approve) or p3 (Reject)
        let t_check_credit = BYawlPatternCompiler::wcp04_exclusive_choice(2, 0b10, 0b1100);

        // Simulate runtime evaluation resulting in an Approval (p2)
        let mut t_check_credit_runtime = t_check_credit;
        t_check_credit_runtime.produce_mask = 0b0100; // Output strictly to p2

        engine.execute_task(&t_check_credit_runtime);
        assert_eq!(engine.state_mask, 0b0100, "Credit check routed to Approval");

        // Task 3: Approve
        let t_approve = BYawlPatternCompiler::wcp01_sequence(3, 0b0100, 0b10000); // p2 -> p4
        engine.execute_task(&t_approve);

        // Task 4: Reject (Never executed in this scenario)
        let _t_reject = BYawlPatternCompiler::wcp01_sequence(4, 0b1000, 0b100000); // p3 -> p5

        // Task 5: Notify Customer (Simple Merge)
        // Consumes from either p4 (Approved) or p5 (Rejected) and outputs to p6
        let t_notify = BYawlPatternCompiler::wcp05_simple_merge(5, 0b110000, 0b1000000);

        let executed_notify = engine.execute_task(&t_notify);
        assert!(
            executed_notify,
            "Simple merge must trigger from the approval path"
        );
        assert_eq!(
            engine.state_mask, 0b1000000,
            "Process completes at the notification state"
        );
    }
}
