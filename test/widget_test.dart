import 'package:flutter_test/flutter_test.dart';
import 'package:dynamodb_manager/src/app.dart';

void main() {
  testWidgets('App builds without errors', (WidgetTester tester) async {
    await tester.pumpWidget(const DynamoDbClientApp());
    expect(find.text('AWS Profiles'), findsOneWidget);
  });
}
