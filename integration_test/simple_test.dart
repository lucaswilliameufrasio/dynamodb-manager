import 'package:flutter_test/flutter_test.dart';
import 'package:dynamodb_manager/src/app.dart';
import 'package:dynamodb_manager/src/rust/frb_generated.dart';
import 'package:integration_test/integration_test.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => await RustLib.init());
  testWidgets('App builds without errors', (WidgetTester tester) async {
    await tester.pumpWidget(const DynamoDbClientApp());
    expect(find.text('AWS Profiles'), findsOneWidget);
  });
}
